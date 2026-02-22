pub const CPU_FREQ: f64 = 1_789_773.0;

const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20,  2, 40,  4, 80,  6,
   160,   8, 60, 10, 14, 12, 26, 14,
    12,  16, 24, 18, 48, 20, 96, 22,
   192,  24, 72, 26, 16, 28, 32, 30,
];

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0], // 12.5%
    [0, 1, 1, 0, 0, 0, 0, 0], // 25%
    [0, 1, 1, 1, 1, 0, 0, 0], // 50%
    [1, 0, 0, 1, 1, 1, 1, 1], // 75% (negated 25%)
];

const TRIANGLE_LOOP: [f32; 32] = [
    15.0, 14.0, 13.0, 12.0, 11.0, 10.0,  9.0,  8.0,  7.0,  6.0,  5.0,  4.0,  3.0,  2.0,  1.0,  0.0,
    0.0,  1.0,  2.0,  3.0,  4.0,  5.0,  6.0,  7.0,  8.0,  9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
];

pub struct Pulse {
    enabled: bool,
    is_pulse_1: bool, // Pulse 1 uses one's-complement negate in sweep; Pulse 2 uses two's-complement.

    length_counter_halt: bool,
    length_counter: u8,

    duty: u8,
    envelope_start: bool,
    envelope_divider: u8,
    envelope_counter: u8,
    constant_volume: bool,
    volume: u8,

    sweep_enabled: bool,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_reload: bool,
    sweep_divider: u8,

    timer_counter: u16,
    timer_period: u16,

    sequencer_step: u8,
}

pub struct Triangle {
    enabled: bool,

    counter_halt: bool,
    length_counter_load: u8,
    length_counter: u8,

    linear_counter: u8,
    linear_counter_load: u8,
    linear_reload: bool,

    timer_counter: u16,
    timer_period: u16,

    sequencer_step: u8,
}

#[allow(dead_code)]
struct Noise {
    enabled: bool,

    length_counter_halt: bool,
    length_counter_load: u8,

    constant_volume: bool,
    envelope: u8,

    mode_flag: bool,

    noise_period: u8,
    noise_mode: bool,
}

#[allow(dead_code)]
struct Dmc {
    enabled: bool,

    irq_enabled: bool,
    irq_loop: bool,
    frequency: u8,

    load_counter: u8,

    sample_adress: u8,
    current_adress: u8,
    sample_length: u8,
    sample_length_counter: u8,
}

pub struct Apu {
    pub pulse_1: Pulse,
    pub pulse_2: Pulse,
    pub triangle: Triangle,
    //pub noise: Noise,
    //pub dmc: Dmc,

    pub frame_counter: u16,
    pub frame_counter_mode: bool,
    pub irq_inhibit: bool,
    pub frame_irq: bool,
}

impl Pulse {
    pub fn new(is_pulse_1: bool) -> Self {
        Pulse {
            enabled: false,
            is_pulse_1,
            length_counter_halt: false,
            length_counter: 0,
            duty: 0,
            envelope_start: false,
            envelope_divider: 0,
            envelope_counter: 0,
            constant_volume: false,
            volume: 0,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_reload: false,
            sweep_divider: 0,
            timer_counter: 0,
            timer_period: 0,
            sequencer_step: 0,
        }
    }

    pub fn write_pulse(&mut self, adress: u16, raw_value: &u8) {
        match adress {
            0x4000 | 0x4004 => {
                self.duty = *raw_value >> 6;
                self.length_counter_halt = *raw_value & 0b0010_0000 != 0;
                self.constant_volume = *raw_value & 0b0001_0000 != 0;
                self.volume = *raw_value & 0b0000_1111;
            },
            0x4001 | 0x4005 => {
                self.sweep_enabled = (*raw_value & 0b1000_0000) != 0;
                self.sweep_period = (*raw_value & 0b0111_0000) >> 4;
                self.sweep_negate = (*raw_value & 0b0000_1000) != 0;
                self.sweep_shift = *raw_value & 0b0000_0111;
                self.sweep_reload = true;
            },
            0x4002 | 0x4006 => {
                self.timer_period = (self.timer_period & 0x700) | (*raw_value as u16);
            },
            0x4003 | 0x4007 => {
                self.timer_period = (self.timer_period & 0x0FF) | ((*raw_value as u16 & 0x07) << 8);
                if self.enabled {
                    self.length_counter = LENGTH_TABLE[(*raw_value >> 3) as usize];
                }
                self.sequencer_step  = 0;
                self.envelope_start  = true;
            },
            _ => panic!("Unknown adress for pulse")
        }
    }

    fn sweep_target(&self) -> u16 {
        let change = self.timer_period >> self.sweep_shift;
        if self.sweep_negate {
            if self.is_pulse_1 {
                self.timer_period.saturating_sub(change + 1)
            } else {
                self.timer_period.saturating_sub(change)
            }
        } else {
            self.timer_period.saturating_add(change)
        }
    }

    pub fn clock_envelope(&mut self) {
        if self.envelope_start {
            self.envelope_start   = false;
            self.envelope_counter = 15;
            self.envelope_divider = self.volume;
        } else if self.envelope_divider == 0 {
            self.envelope_divider = self.volume;
            if self.envelope_counter > 0 {
                self.envelope_counter -= 1;
            } else if self.length_counter_halt {
                self.envelope_counter = 15;
            }
        } else {
            self.envelope_divider -= 1;
        }
    }

    pub fn clock_length(&mut self) {
        if !self.length_counter_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    pub fn clock_sweep(&mut self) {
        let target = self.sweep_target();

        if self.sweep_divider == 0 {
            if self.sweep_enabled && self.sweep_shift > 0
                && self.timer_period >= 8 && target <= 0x7FF
            {
                self.timer_period = target;
            }
        }

        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload  = false;
        } else {
            self.sweep_divider -= 1;
        }
    }

    pub fn tick(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_period;
            self.sequencer_step = (self.sequencer_step + 1) & 0b111;
        } else {
            self.timer_counter -= 1;
        }
    }

    pub fn output(&self) -> f32 {
        if !self.enabled
            || self.length_counter == 0
            || self.timer_period < 8
            || self.sweep_target() > 0x7FF
        {
            return 0.0;
        }

        if DUTY_TABLE[self.duty as usize][self.sequencer_step as usize] == 0 {
            return 0.0;
        }

        if self.constant_volume { self.volume as f32 } else { self.envelope_counter as f32 }
    }
}

impl Triangle {
    pub fn write_triangle(&mut self,adress: u16, raw_value: &u8) {
        match adress {
            0x4008 => {
                self.counter_halt = *raw_value & 0b1000_000 != 0;
                self.linear_counter_load = *raw_value & !0b1000_000;
            },
            0x400A => self.timer_period = *raw_value as u16,
            0x400B => {
                self.length_counter_load = *raw_value  >> 3;
                self.timer_period = (self.timer_period & 0x0FF) | ((*raw_value as u16 & 0x07) << 8);
            },
            _ => panic!("Unknown adress for triangle"),
        }
    }

    pub fn new() -> Self {
        return Triangle {
            enabled: false,
            counter_halt: false,
            length_counter_load: 0,
            linear_counter_load: 0,
            timer_period: 0,
            length_counter: 0,
            linear_counter: 0,
            sequencer_step: 0,
            linear_reload: false,
            timer_counter: 0,
        }
    }

    pub fn clock_length(&mut self) {
        if !self.counter_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    pub fn clock_linear(&mut self) {
        if self.linear_reload {
            self.linear_counter = self.linear_counter_load;
        }
        if !self.counter_halt && self.linear_counter > 0 {
            self.linear_counter -= 1;
        }
    }

    pub fn output(&self) -> f32 {
        if !self.enabled ||
            (self.timer_period < 2) {
                return 0.0;
        }
        return TRIANGLE_LOOP[self.sequencer_step as usize];
    }

    pub fn tick(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_period;
            self.sequencer_step = (self.sequencer_step + 1) & 0b111;
        } else {
            self.timer_counter -= 1;
        }
    }
}

impl Noise {
    #[allow(dead_code)]
    pub fn write_noise(&mut self, adress: u16, raw_value: &u8) {
        match adress {
            0x400C => {
                self.length_counter_halt = *raw_value & 0b10_0000 != 0;
                self.constant_volume = *raw_value & 0b1_0000 != 0;
                self.envelope = *raw_value & 0b1111;
            },
            0x400E => {
                self.noise_mode = *raw_value & 0b1000_0000 != 0;
                self.noise_period = *raw_value & 0b1111;
            },
            0x400F => {
                self.length_counter_load = (*raw_value >> 3) & 0b1_1111;
            },
            _ => panic!("Unknown adress for noise.")
        }
    }
}

impl Dmc {
    #[allow(dead_code)]
    pub fn write_dmc(&mut self, adress: u16, raw_value: &u8) {
        match adress {
            0x4010 => {
                self.irq_enabled = *raw_value & 0b1000_0000 != 0;
                self.irq_loop = *raw_value & 0b100_0000 != 0;
                self.frequency = *raw_value & 0b1111;
            },
            0x4011 => {
                self.load_counter = *raw_value & 0b111_1111;
            },
            0x4012 => {
                self.sample_adress = *raw_value;
            },
            0x4013 => {
                self.sample_length = *raw_value;
            },
            _ => panic!("Unknow adress for dmc"),
        }
    }
}

impl Apu {
    pub fn new() -> Self {
        Apu {
            pulse_1: Pulse::new(true),
            pulse_2: Pulse::new(false),
            triangle: Triangle::new(),
            frame_counter: 0,
            frame_counter_mode: false,
            irq_inhibit: false,
            frame_irq: false,
        }
    }

    pub fn write_status(&mut self, value: u8) {
        self.pulse_1.enabled = value & 0b01 != 0;
        if !self.pulse_1.enabled { self.pulse_1.length_counter = 0; }

        self.pulse_2.enabled = value & 0b10 != 0;
        if !self.pulse_2.enabled { self.pulse_2.length_counter = 0; }
    }

    pub fn read_status(&mut self) -> u8 {
        let mut result: u8 = 0;

        if self.pulse_1.length_counter > 0 { result |= 0b01; }
        if self.pulse_2.length_counter > 0 { result |= 0b10; }
        if self.frame_irq { result |= 0b0100_0000; }
        self.frame_irq = false;

        result
    }

    pub fn write_frame_counter(&mut self, value: u8) {
        self.frame_counter_mode = value & 0b1000_0000 != 0;
        self.irq_inhibit        = value & 0b0100_0000 != 0;
        if self.irq_inhibit { self.frame_irq = false; }
        if self.frame_counter_mode {
            self.clock_quarter_frame();
            self.clock_half_frame();
        }
    }

    pub fn clock_frame_counter(&mut self) {
        self.frame_counter += 1;

        if self.frame_counter_mode {
            match self.frame_counter {
                3729  => self.clock_quarter_frame(),
                7457  => { self.clock_quarter_frame(); self.clock_half_frame(); },
                11186 => self.clock_quarter_frame(),
                14915 => {},
                18641 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                    self.frame_counter = 0;
                },
                _ => {}
            }
        } else {
            match self.frame_counter {
                3729  => self.clock_quarter_frame(),
                7457  => { self.clock_quarter_frame(); self.clock_half_frame(); },
                11186 => self.clock_quarter_frame(),
                14915 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                    if !self.irq_inhibit { self.frame_irq = true; }
                    self.frame_counter = 0;
                },
                _ => {},
            }
        }
    }

    pub fn clock_quarter_frame(&mut self) {
        self.pulse_1.clock_envelope();
        self.pulse_2.clock_envelope();
        self.triangle.clock_linear();
    }

    pub fn clock_half_frame(&mut self) {
        self.pulse_1.clock_length();
        self.pulse_1.clock_sweep();
        self.pulse_2.clock_length();
        self.pulse_2.clock_sweep();
        self.triangle.clock_length();
    }

    pub fn tick(&mut self) {
        self.pulse_1.tick();
        self.pulse_2.tick();
        self.triangle.tick();
    }

    pub fn sample(&self) -> f32 {
        let p1 = self.pulse_1.output();
        let p2 = self.pulse_2.output();
        let t = self.triangle.output();
        let pulse_out = if p1 + p2 == 0.0 { 0.0 } else { 95.88 / (8128.0 / (p1 + p2) + 100.0) };
        let tnd_out = 159.79 / (1.0 / (t / 8227.0) + 100.0);

        return pulse_out + tnd_out;
    }
}

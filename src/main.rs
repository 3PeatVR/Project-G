mod cpu;
use cpu::Cpu;

fn main() {
   fn test_lda_from_memory() {
       let mut cpu = Cpu::new();
       cpu.mem_write(0x55, 0x10);

       cpu.load_and_run_pg(vec![0xa5, 0x10, 0x00]);

       assert_eq!(cpu.register_a, 0x55);
   }
   test_lda_from_memory();
}

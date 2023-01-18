use std::env::args;
use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};
use rand::prelude::*;

mod opcodes;


fn main() {
    let file_path = args().nth(1);
    match file_path {
        None => panic!("No file path"),
        Some(file_path) => {
            let mut file = File::open(file_path).unwrap();
            let mut data = Vec::new();
            let len = file.read_to_end(&mut data).unwrap();

            let mut chip8 = Chip8::new();
            chip8.load_rom(data);
            loop {
                chip8.run_next();
            }
        }
    }
}

/// 12 bit address pointer
type Address = u16;

#[derive(Debug)]
struct Chip8 {
    /// 4K memory
    memory: [u8; 4096],
    /// registers
    v: [u8; 16],
    /// current address
    i: Address,
    /// program counter
    pc: Address,
    /// stack (return addresses)
    stack: Vec<Address>,
    /// delay timer (60Hz, when non-zero, decremented at 60Hz)
    delay_timer: u8,
    /// sound timer (when non-zero, beeps, and decrements at 60Hz)
    sound_timer: u8,
    /// graphics (64x32 black and white)
    display: [bool; 64 * 32],
    start_time: Instant,
    last_processed_timers: Instant
}

const WIDTH: usize = 32;
const HEIGHT: usize = 64;

impl Chip8 {
    fn new() -> Chip8 {
        let mut c = Chip8 {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: 0x100,
            stack: Vec::new(),
            delay_timer: 0,
            sound_timer: 0,
            display: [false; 64 * 32],
            start_time: Instant::now(),
            last_processed_timers: Instant::now()
        };

        // initialize font
        c.memory[0..80].clone_from_slice(&[
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80  // F
        ]);

        c
    }

    fn load_rom(&mut self, rom: Vec<u8>) {
        self.memory[0x200..0x200+rom.len()].clone_from_slice(&*rom);
    }

    fn run_next(&mut self) {
        let current = (self.memory[self.pc as usize * 2] as u16) << 8 | (self.memory[self.pc as usize * 2 + 1] as u16);
        let first  = ((current & 0xF000) as u16 >> 12) as u8;
        let first_raw = (current & 0xF000) as u16;
        let second  = ((current & 0x0F00 as u16) >> 8 as u16) as u8;
        let second_raw = (current & 0x0F00) as u16;
        let third   = ((current & 0x00F0) as u16 >> 4) as u8;
        let third_raw = (current & 0x00F0) as u16;
        let fourth  = (current & 0x000F) as u8;
        let fourth_raw = (current & 0x000F) as u16;
        let last_two = (third_raw|fourth_raw) as u8;
        let after_first = current ^ (first_raw as Address);

        print!("{:#x}", self.pc);

        if current == 0 {
            dbg!(&self);
            panic!("Null call {}", self.pc);
        }

        // println!("{current:#04x} -> F{first_raw:#x} S{second_raw:#x} T{third_raw:#x} F{fourth_raw:#x} - !F={after_first:#x} {last_two:#x}");

        if first == 0 {
            // syscalls
            if current == 0x00e0 {
                println!("CLS");
                self.display = [false; 64 * 32];
            } else if current == 0x00ee {
                if self.stack.is_empty() {
                    panic!("Stack underflow");
                }
                self.pc = self.stack.pop().unwrap();
                return;
            } else {
                println!("SYS {}", after_first);
            }
        } else if first == 1 {
            // jump
            println!("JP {after_first:#x} ({after_first})");
            if after_first / 2 == self.pc {
                panic!("HALT! Jump to self (direct infinite loop)");
            }
            self.pc = after_first / 2;
            return;
        } else if first == 2 {
            // call
            println!("CALL {}", after_first);
            self.stack.push(self.pc);
            self.pc = after_first;
        } else if first == 3 {
            // if eq
            println!("SE IF V{second} == {}", last_two);
            if self.v[second as usize] != last_two {
                self.pc += 1;
            }
        } else if first == 4 {
            // if not eq
            println!("SNE IF V{second} != {}", last_two);
            if self.v[second as usize] == last_two {
                self.pc += 1;
            }
        } else if first == 5 {
            // if Vx == Vy
            if self.v[second as usize] != self.v[third as usize] {
                self.pc += 1;
            }
        } else if first == 6 {
            // LOAD; Vx = NN
            println!("LD V{second} = {last_two:#x} ({last_two})");
            self.v[second as usize] = last_two;
        } else if first == 7 {
            // ADD; Vx += NN
            println!("ADD V{second} += {last_two:#x} ({last_two})");
            self.v[second as usize] += last_two;
        } else if first == 8 {
            println!("eights");
            if fourth == 0 {
                self.v[second as usize] = self.v[third as usize];
            } else if fourth == 1 {
                self.v[second as usize] |= self.v[third as usize];
            } else if fourth == 2 {
                self.v[second as usize] &= self.v[third as usize];
            } else if fourth == 3 {
                self.v[second as usize] ^= self.v[third as usize];
            } else if fourth == 4 {
                self.v[second as usize] += self.v[third as usize];
                // TODO! Carry implementation
            } else if fourth == 5 {
                self.v[second as usize] -= self.v[third as usize];
                // TODO! Borrow implementation
            }  else if fourth == 6 {
                self.v[0xF] = self.v[second as usize] & 1;
                self.v[second as usize] >>= 1;
            } else if fourth == 7 {
                self.v[second as usize] = self.v[third as usize] - self.v[second as usize];
            } else if fourth == 0xE {
                self.v[0xF] = if (self.v[second as usize] & 0b10000000) != 0 { 1 } else { 0};
                self.v[second as usize] <<= 1;
            }
        } else if first == 9 {
            // if Vx != Vy
            println!("IF V{second} != V{third}");
            if self.v[second as usize] == self.v[third as usize] {
                self.pc += 1;
            }
        } else if first == 0xA {
            // I=NNN
            println!("I={after_first:#x} ({after_first})");
            self.i = after_first;
        } else if first == 0xB {
            println!("JP+ {} {after_first:#x} ({after_first})", self.v[0]);
            self.pc = (self.v[0] as Address) + after_first;
            return;
        } else if first == 0xC {
            // I=rand() & NN
            println!("RND V{second} = random & {last_two:#x} ({last_two} {last_two:#b})");
            let random: u8 = random();
            self.v[second as usize] = random & last_two;
        } else if first == 0xD {
            // draw(Vx, Vy, N)

            // draws at Vx, Vy, width 8, height N
            // sprite (bit coded XOR (set bits flip the bit value)) from I
            // VF set to 1 if any bit is set to 0

            println!("DRW X=V{second} Y=V{third} W=16 H={fourth}");

            let x = self.v[second as usize] % (WIDTH as u8);
            let y = self.v[third as usize] % (HEIGHT as u8);
            let sprite_height = fourth as usize;

            self.v[0xF] = 0;

            for sprite_y in 1..sprite_height+1 {
                let sprite_y = (sprite_height - sprite_y + y as usize) % (HEIGHT);
                let sprite_row = self.memory[self.i as usize + sprite_y];
                println!("\n{:#b} ({:#x})", sprite_row, sprite_row);
                for b in 0..8 {
                    let mut pos: usize = (sprite_y as usize * WIDTH) + ((b as usize) + (x as usize) % WIDTH as usize);
                    if (x as i16) + (b as i16) < 0 { pos += WIDTH - 1 }
                    // let orig = self.display[pos];
                    self.display[pos] ^= (sprite_row & (1 << (7-b))) > 0;
                    // println!("pos{} x{} y{}", pos, pos % WIDTH, pos / WIDTH);
                    // println!("x{x} + y{y}*16 + {b} = {pos} .. {:#010b} = {:#010b}", 1 << (7-b), (memory & (1 << (7-b))));
                    // print!("{}", if (sprite_row & (1 << (7-b))) > 0 { "█" } else { " " });
                    // println!("");
                    if self.v[0xF] == 0 {
                        self.v[0xF] = if self.display[pos] && (sprite_row & (1 << (7-b))) > 0 { 1 } else { 0 };
                    }
                }
            }

            for x in 0..WIDTH {
                for y in 0..HEIGHT {
                    print!("{}", if self.display[y*WIDTH + x] { "█" } else { " " });
                }
                println!();
            }
        } else if first == 0xE {
            // Keyboard operations

            // TODO! implement keyboard

            if last_two == 0x9E {
                // if key() == Vx
                println!("key() == V{last_two}");
            } else if last_two == 0xA1 {
                // if key() != Vx
                println!("key() != V{last_two}");
            }
        } else if first == 0xF {
            if last_two == 0x07 {
                // Vx = get_delay()
                println!("V{second} = delay()");
                self.v[second as usize] = self.delay_timer;
            } else if last_two == 0x0A {
                // Vx = get_key()
                println!("V{second} = key()");
                self.v[second as usize] = 0; // TODO! implement keyboard
            } else if last_two == 0x15 {
                println!("delay(V{second})");
                self.delay_timer = self.v[second as usize];
            } else if last_two == 0x18 {
                println!("sound({second})");
                self.sound_timer = self.v[second as usize];
            } else if last_two == 0x1E {
                println!("I += V{second}");
                self.i += self.v[second as usize] as u16;
            } else if last_two == 0x29 {
                // I = sprite_addr[Vx]
                // Characters 0x0-0xF are represented by a 4x5 font
                println!("I=sprite_addr[V{second}]");
                self.i = (4 * 5 * second) as Address;
            } else if last_two == 0x33 {
                // Stores binary-coded decimal representation of Vx, hundreds digit at I, tens digit at I+1 and ones digit at I+2
                let num = self.v[second as usize];
                self.memory[self.i as usize] = num / 100;
                self.memory[self.i as usize + 1] = num / 10 % 100;
                self.memory[self.i as usize + 2] = num % 10;
                println!("encoded I = V{second}");
            } else if last_two == 0x55 {
                // reg_dump(Vx, &I)
                println!("reg_dump(V{second}, I)");
                if (self.i+second as u16) as usize >= self.memory.len() {
                    panic!("overflow with reg_dump");
                }
                self.memory[self.i as usize..second as usize].clone_from_slice(&self.v[0..second as usize])
            } else if last_two == 0x65 {
                // reg_load(Vx, &I)
                println!("reg_load(V{second}, I)");
                if (self.i+second as u16) as usize >= self.memory.len() {
                    panic!("overflow with reg_load");
                }
                self.v[0..second as usize].clone_from_slice(&self.memory[self.i as usize..second as usize]);
            }
        }
        self.pc += 1;
        if self.pc as usize > self.memory.len() {
            panic!("Halted (Program counter over ROM length)");
        }
        let now = Instant::now();
        if now - self.last_processed_timers > Duration::from_millis(16) {
            if self.sound_timer > 0 {
                print!("\x07");
                self.sound_timer -= 1;
            }
            if self.delay_timer > 0 {
                self.delay_timer -= 1;
            }
        }
    }
}
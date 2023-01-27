use std::env::args;
use std::fs::File;
use std::io::{Read, Write};
use std::thread;
use std::time::{Duration, Instant};
use rand::prelude::*;
use debug_print::{debug_eprint, debug_eprintln};

enum Flags {
    RenderOnClsOnly = 1,
    DelayWait = 2
}

fn main() {
    let file_path = args().nth(1);
    match file_path {
        None => panic!("No file path"),
        Some(file_path) => {
            let mut file = File::open(file_path).unwrap();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();

            let mut chip8 = Chip8::new();
            chip8.load_rom(data);

            if args().any(|x| x == "--render-on-cls-only") {
                chip8.flags |= Flags::RenderOnClsOnly as u8;
            }
            if args().any(|x| x == "--delay-wait") {
                chip8.flags |= Flags::DelayWait as u8;
            }

            loop {
                chip8.run_next();
            }
        }
    }
}

/// 12 bit address pointer
type Address = u32;

#[derive(Debug)]
struct Chip8 {
    /// 4K memory
    memory: Vec<u8>, // 2^18
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
    display: [[bool; HEIGHT]; WIDTH],
    // start_time: Instant,
    last_processed_timers: Instant,

    display_changed: bool,

    flags: u8
}

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

impl Chip8 {
    fn new() -> Chip8 {
        let mut c = Chip8 {
            memory: Vec::with_capacity(4096),
            v: [0; 16],
            i: 0,
            pc: 0x200,
            stack: Vec::new(),
            delay_timer: 0,
            sound_timer: 0,
            display: [[false; HEIGHT]; WIDTH],
            // start_time: Instant::now(),
            last_processed_timers: Instant::now(),
            display_changed: false,
            flags: 0
        };

        // initialize font
        c.memory.write_all(&[
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
        ]).expect("Failed to write font");

        c.memory.resize(4096, 0);

        c
    }

    fn load_rom(&mut self, rom: Vec<u8>) {
        self.memory.resize(0x200, 0);
        self.memory.extend_from_slice(&rom);
    }

    fn run_next(&mut self) {
        let current = (self.memory[self.pc as usize] as u16) << 8 | (self.memory[self.pc as usize + 1] as u16);
        let first  = ((current & 0xF000) as u16 >> 12) as u8;
        let first_raw = (current & 0xF000) as u16;
        let second  = ((current & 0x0F00 as u16) >> 8 as u16) as u8;
        // let second_raw = (current & 0x0F00) as u16;
        let third   = ((current & 0x00F0) as u16 >> 4) as u8;
        let third_raw = (current & 0x00F0) as u16;
        let fourth  = (current & 0x000F) as u8;
        let fourth_raw = (current & 0x000F) as u16;
        let last_two = (third_raw|fourth_raw) as u8;
        let after_first = current ^ (first_raw as u16);

        debug_eprint!("{:#x} ", self.pc);

        if current == 0 {
            // dbg!(&self);
            let mut file = File::create("dump").unwrap();
            file.write_all(&self.memory).unwrap();
            panic!("Null call {:#x} ({})", self.pc, self.pc);
        }

        // println!("{current:#04x} -> F{first_raw:#x} S{second_raw:#x} T{third_raw:#x} F{fourth_raw:#x} - !F={after_first:#x} {last_two:#x}");

        if first == 0 {
            // syscalls
            if current == 0x00e0 {
                debug_eprintln!("CLS");
                if self.display_changed {
                    self.render();
                }
                self.display = [[false; HEIGHT]; WIDTH];
            } else if current == 0x00ee {
                if self.stack.is_empty() {
                    panic!("Stack underflow");
                }
                self.pc = self.stack.pop().unwrap();
                return;
            } else {
                eprintln!("SYS {}", after_first);
            }
        } else if first == 1 {
            // jump
            debug_eprintln!("JP {after_first:#x} ({after_first})");
            if after_first == self.pc as u16 {
                panic!("HALT! Jump to self (direct infinite loop)");
            }
            self.pc = after_first as Address;
            return;
        } else if first == 2 {
            // call
            debug_eprintln!("CALL {after_first} {after_first:#02x}");
            self.stack.push(self.pc);
            self.pc = after_first as Address;
            return;
        } else if first == 3 {
            // if eq
            debug_eprintln!("SE IF V{second} == {} SKIP (={})", last_two, self.v[second as usize]);
            if self.v[second as usize] == last_two {
                self.pc += 2;
            }
        } else if first == 4 {
            // if not eq
            debug_eprintln!("SNE IF V{second} != {} SKIP (={})", last_two, self.v[second as usize]);
            if self.v[second as usize] != last_two {
                self.pc += 2;
            }
        } else if first == 5 {
            // if Vx == Vy
            if self.v[second as usize] != self.v[third as usize] {
                self.pc += 2;
            }
        } else if first == 6 {
            // LOAD; Vx = NN
            debug_eprintln!("LD V{second} = {last_two:#02x} ({last_two})");
            self.v[second as usize] = last_two;
        } else if first == 7 {
            // ADD; Vx += NN
            debug_eprintln!("ADD V{second} += {last_two:#x} ({last_two}) => {:#x} ({})", self.v[second as usize] + last_two, self.v[second as usize] + last_two);
            self.v[second as usize] += last_two;
        } else if first == 8 {
            debug_eprintln!("eights");
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
                self.v[0xF] = if self.v[second as usize] as u16 + self.v[third as usize] as u16 > 255 { 1 } else { 0 };
            } else if fourth == 5 {
                self.v[second as usize] -= self.v[third as usize];
                self.v[0xF] = if self.v[second as usize] > self.v[third as usize] { 1 } else { 0 }
            }  else if fourth == 6 {
                self.v[0xF] = self.v[second as usize] & 1;
                self.v[second as usize] >>= 1;
            } else if fourth == 7 {
                self.v[second as usize] = self.v[third as usize] - self.v[second as usize];
                self.v[0xF] = if self.v[third as usize] > self.v[second as usize] { 1 } else { 0 }
            } else if fourth == 0xE {
                self.v[0xF] = if (self.v[second as usize] & 0b10000000) != 0 { 1 } else { 0};
                self.v[second as usize] <<= 1;
            }
        } else if first == 9 {
            // if Vx != Vy
            debug_eprintln!("IF V{second} != V{third}");
            if self.v[second as usize] == self.v[third as usize] {
                self.pc += 2;
            }
        } else if first == 0xA {
            // I=NNN
            debug_eprintln!("LD I = {after_first:#x} ({after_first})");
            self.i = after_first as Address;
        } else if first == 0xB {
            debug_eprintln!("JP+ {} {after_first:#x} ({after_first})", self.v[0]);
            self.pc = (self.v[0] as Address) + after_first as Address;
            return;
        } else if first == 0xC {
            // I=rand() & NN
            debug_eprintln!("RND V{second} = random & {last_two:#x} ({last_two} {last_two:#b})");
            let random: u8 = random();
            self.v[second as usize] = random & last_two;
        } else if first == 0xD {
            // draw(Vx, Vy, N)

            // draws at Vx, Vy, width 8, height N
            // sprite (bit coded XOR (set bits flip the bit value)) from I
            // VF set to 1 if any bit is set to 0

            debug_eprintln!("DRW X=V{second} Y=V{third} W=16 H={fourth} I={:#x} ({})", self.i, self.i);

            let x = self.v[second as usize] % (WIDTH as u8);
            let y = self.v[third as usize] % (HEIGHT as u8);
            let sprite_height = fourth as usize;

            debug_eprintln!("X={x} Y={y}");

            self.v[0xF] = 0;

            for sprite_y in 0..sprite_height {
                let sprite_y = sprite_y;
                let sprite_row = self.memory[self.i as usize + sprite_y];
                let y = (sprite_y + y as usize) % HEIGHT;
                // debug_eprintln!("\n{:#b} ({:#x})", sprite_row, sprite_row);
                for b in 0..8 {
                    let x = (x as usize + b as usize) % WIDTH;
                    // println!("x{x} y{y}");
                    self.display[x][y] ^= (sprite_row & (1 << (7-b))) > 0;
                    // debug_eprint!("{}", if (sprite_row & (1 << (7-b))) > 0 { "█" } else { " " });
                    // println!("");
                    if self.v[0xF] == 0 {
                        self.v[0xF] = if self.display[x][y] && (sprite_row & (1 << (7-b))) > 0 { 1 } else { 0 };
                    }
                }
            }

            self.display_changed = true;
        } else if first == 0xE {
            // Keyboard operations

            // TODO! implement keyboard

            if last_two == 0x9E {
                // if key() == Vx
                debug_eprintln!("key() == V{last_two}");
            } else if last_two == 0xA1 {
                // if key() != Vx
                debug_eprintln!("key() != V{last_two}");
            }
        } else if first == 0xF {
            if last_two == 0x07 {
                // Vx = get_delay()
                debug_eprintln!("V{second} = delay() => {:#x} ({})", self.delay_timer, self.delay_timer);
                self.v[second as usize] = self.delay_timer;
            } else if last_two == 0x0A {
                // Vx = get_key()
                debug_eprintln!("V{second} = key()");
                self.v[second as usize] = 0; // TODO! implement keyboard
            } else if last_two == 0x15 {
                debug_eprintln!("delay(V{second}) (V{second}={:#x}={})", self.v[second as usize], self.v[second as usize]);
                self.delay_timer = self.v[second as usize];
                if self.flags & Flags::DelayWait as u8 != 0 {
                    spin_sleep::sleep(Duration::from_millis((1000 / 60) * (self.v[second as usize] as u64)));
                    self.delay_timer = 0;
                }
            } else if last_two == 0x18 {
                debug_eprintln!("sound({second})");
                self.sound_timer = self.v[second as usize];
            } else if last_two == 0x1E {
                debug_eprintln!("I += V{second} ({:#x}; {})", self.i + self.v[second as usize] as Address, self.i + self.v[second as usize] as Address);
                self.i += self.v[second as usize] as Address;
            } else if last_two == 0x29 {
                // I = sprite_addr[Vx]
                // Characters 0x0-0xF are represented by a 8x5 font
                debug_eprintln!("I=sprite_addr[V{second}]");
                self.i = (4 * 5 * second) as Address;
            } else if last_two == 0x33 {
                // Stores binary-coded decimal representation of Vx, hundreds digit at I, tens digit at I+1 and ones digit at I+2
                let num = self.v[second as usize];
                self.memory[self.i as usize] = num / 100;
                self.memory[self.i as usize + 1] = num / 10 % 100;
                self.memory[self.i as usize + 2] = num % 10;
                debug_eprintln!("encoded I = V{second}");
            } else if last_two == 0x55 {
                // reg_dump(Vx, &I)
                debug_eprintln!("reg_dump(V{second}, I)");
                if (self.i+second as Address) as usize >= self.memory.len() {
                    panic!("overflow with reg_dump");
                }
                self.memory[self.i as usize..second as usize].clone_from_slice(&self.v[0..second as usize])
            } else if last_two == 0x65 {
                // reg_load(Vx, &I)
                debug_eprintln!("reg_load(V{second}, I)");
                if (self.i+second as Address) as usize >= self.memory.len() {
                    panic!("overflow with reg_load");
                }
                self.v[0..second as usize].clone_from_slice(&self.memory[self.i as usize..second as usize]);
            }
        }
        self.pc += 2;
        if self.pc as usize > self.memory.len() {
            panic!("Halted (Program counter over ROM length)");
        }
        let now = Instant::now();
        if now - self.last_processed_timers >= Duration::from_millis(15) {
            debug_eprintln!("Processing timers (duration: {:?})", now - self.last_processed_timers);
            if self.sound_timer > 0 {
                print!("\x07");
                self.sound_timer -= 1;
            }
            if self.delay_timer > 0 {
                debug_eprint!("delay_timer: {} - 1 => ", self.delay_timer);
                self.delay_timer -= 1;
                debug_eprintln!("{}", self.delay_timer);
            }
            self.last_processed_timers = now;

            if self.display_changed && self.flags & Flags::RenderOnClsOnly as u8 == 0 { self.render(); }
        } else {
            // println!("Skipping timers, duration: {:?}", now - self.last_processed_timers);
        }
        // thread::sleep(Duration::from_micros(50));
    }

    fn render(&mut self) {
        debug_eprintln!("\n-----------------------------------\n");

        for y in 0..HEIGHT {
        for x in 0..WIDTH {
        // println!("x{x} y{y}");
        print!("{}", if self.display[x][y] { "█" } else { " " });
        }
        println!();
        }
        debug_eprintln!("\n-----------------------------------\n");

        self.display_changed = false;
    }
}
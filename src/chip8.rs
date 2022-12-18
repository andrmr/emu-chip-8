type Byte = u8;
type Word = u16;

pub const SCREEN_WIDTH: u8 = 64;
pub const SCREEN_HEIGHT: u8 = 32;

const MEM_COUNT: usize = 4096;
const REG_COUNT: usize = 16;
const STACK_COUNT: usize = 16;
const CARRY_REG: usize = 0xF;

const PROGRAM_OFFSET: usize = 0x200;

const FONT_OFFSET: usize = 0x50;
const FONT_SIZE: usize = 5;
const FONT_COUNT: usize = 16;
const FONT_SET: [Byte; FONT_SIZE * FONT_COUNT] = [
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
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Chip8 {
    pub display: [[bool; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    memory: [Byte; MEM_COUNT],
    registers: [Byte; REG_COUNT],
    index: Word,
    program_counter: Word,
    stack: arrayvec::ArrayVec<Word, STACK_COUNT>,
    delay_timer: Byte,
    sound_timer: Byte,
    op: Word,
    pub key: u8,
}

impl Chip8 {
    pub fn new() -> Self {
        let mut chip8 = Self {
            display: [[false; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
            memory: [0; MEM_COUNT],
            registers: [0; REG_COUNT],
            index: 0,
            program_counter: PROGRAM_OFFSET as Word,
            stack: arrayvec::ArrayVec::new(),
            delay_timer: 0,
            sound_timer: 0,
            op: 0,
            key: 0,
        };

        chip8.load_fonts();

        chip8
    }

    pub fn load_rom(&mut self, rom: &str) -> Result<(), std::io::Error> {
        let file = std::fs::File::open(rom)
            .expect("Unable to read ROM");

        let mut reader = std::io::BufReader::new(file);
        let loaded = std::io::Read::read(&mut reader, &mut self.memory[PROGRAM_OFFSET..])
            .expect("Unable to load ROM");

        println!("Loaded {} bytes", loaded);

        Ok(())
    }

    fn load_fonts(&mut self) {
        for (i, &byte) in FONT_SET.iter().enumerate() {
            self.memory[FONT_OFFSET + i] = byte;
        }
    }

    fn decrement_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn handle_op(&mut self) {
        self.decrement_timers();

        self.op = (self.memory[self.program_counter as usize] as Word) << 8
            | self.memory[(self.program_counter + 1) as usize] as Word;

        println!("OP: {:#X}", self.op);

        let mut jump = false;

        match (self.op & 0xF000) >> 12
        {
            0x0 => self.clear_or_return(),
            0x1 => { self.jump_addr(); jump = true; },
            0xB => { self.jump_addr_offset(); jump = true; },
            0x2 => { self.call_addr(); jump = true; },
            0x3 => self.skip_eq_byte(),
            0x4 => self.skip_ne_byte(),
            0x5 => self.skip_eq_reg(),
            0x9 => self.skip_ne_reg(),
            0x6 => self.load_byte(),
            0x7 => self.add_byte(),
            0x8 => self.logical_op(),
            0xA => self.set_index(),
            0xC => self.rand(),
            0xD => self.draw(),
            0xE => self.keyboard_op(),
            0xF => self.misc_op(),
            _ => eprintln!("Unhandled opcode: {:#X}", self.op)
        }

        if !jump {
            self.program_counter += 2;
        }
    }

    pub fn clear_or_return(&mut self) {
        match self.op & 0x00FF {
            0xE0 => self.display = [[false; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
            0xEE => self.program_counter = self.stack.pop().expect("Stack underflow"),
            _ => eprintln!("Unknown opcode: {:#X}", self.op),
        }
    }

    fn jump_addr(&mut self) {
        self.program_counter = self.op & 0x0FFF;
    }

    fn jump_addr_offset(&mut self) {
        self.program_counter = (self.op & 0x0FFF) + self.registers[0] as Word;
    }

    fn call_addr(&mut self) {
        self.stack.push(self.program_counter);
        self.program_counter = self.op & 0x0FFF;
    }

    fn skip_eq_byte(&mut self) {
        let x = ((self.op & 0x0F00) >> 8) as usize;
        let byte = (self.op & 0x00FF) as Byte;

        if self.registers[x] == byte {
            self.program_counter += 2;
        }
    }

    fn skip_ne_byte(&mut self) {
        let x = ((self.op & 0x0F00) >> 8) as usize;
        let byte = (self.op & 0x00FF) as Byte;

        if self.registers[x] != byte {
            self.program_counter += 2;
        }
    }

    fn skip_eq_reg(&mut self) {
        let x = ((self.op & 0x0F00) >> 8) as usize;
        let y = ((self.op & 0x00F0) >> 4) as usize;

        if self.registers[x] == self.registers[y] {
            self.program_counter += 2;
        }
    }

    fn skip_ne_reg(&mut self) {
        let x = ((self.op & 0x0F00) >> 8) as usize;
        let y = ((self.op & 0x00F0) >> 4) as usize;

        if self.registers[x] != self.registers[y] {
            self.program_counter += 2;
        }
    }

    /// 6xkk - LD Vx, byte
    /// Set Vx = kk.
    /// The interpreter puts the value kk into register Vx.
    fn load_byte(&mut self) {
        let reg = ((self.op & 0x0F00) >> 8) as usize;
        let val = (self.op & 0x00FF) as Byte;

        self.registers[reg] = val;
    }

    /// 7xkk - ADD Vx, byte
    /// Set Vx = Vx + kk.
    /// Adds the value kk to the value of register Vx, then stores the result in Vx.
    fn add_byte(&mut self) {
        let reg = ((self.op & 0x0F00) >> 8) as usize;
        let val = (self.op & 0x00FF) as Byte;

        self.registers[reg] = self.registers[reg].wrapping_add(val);
            // .expect("Overflow in add op");
    }

    /// 8xyB - B Vx, Vy
    /// Performs bitwise B on Vx and Vy
    /// B is a value from 0x0 to 0xE
    fn logical_op(&mut self) {
        println!("Logical op: {:#X}", self.op);

        let x = ((self.op & 0x0F00) >> 8) as usize;
        let y = ((self.op & 0x00F0) >> 4) as usize;

        match self.op & 0x000F {
            // 8xy0 - LD Vx, Vy
            // Set Vx = Vy.
            // Stores the value of register Vy in register Vx.
            0x0 => self.registers[x] = self.registers[y],

            // 8xy1 - OR Vx, Vy
            // Set Vx = Vx OR Vy.
            // Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
            // A bitwise OR compares the corresponding bits from two values, and if either bit is 1,
            // then the same bit in the result is also 1. Otherwise, it is 0.
            0x1 => self.registers[x] |= self.registers[y],

            // 8xy2 - AND Vx, Vy
            // Set Vx = Vx AND Vy.
            // Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.
            // A bitwise AND compares the corresponding bits from two values, and if both bits are 1,
            // then the same bit in the result is also 1. Otherwise, it is 0.
            0x2 => self.registers[x] &= self.registers[y],

            // 8xy3 - XOR Vx, Vy
            // Set Vx = Vx XOR Vy.
            // Performs a bitwise exclusive OR on the values of Vx and Vy, then stores the result in Vx.
            // An exclusive OR compares the corresponding bits from two values, and if the bits are not both the same,
            // then the corresponding bit in the result is set to 1. Otherwise, it is 0.
            0x3 => self.registers[x] ^= self.registers[y],

            // 8xy4 - ADD Vx, Vy
            // Set Vx = Vx + Vy, set VF = carry.
            // The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e., > 255,) VF is set to 1, otherwise 0.
            // Only the lowest 8 bits of the result are kept, and stored in Vx.
            0x4 => {
                let overflow = self.registers[x].checked_add(self.registers[y]).is_none();
                self.registers[x] = self.registers[x].wrapping_add(self.registers[y]);
                self.registers[CARRY_REG] = overflow as Byte;
            },

            // 8xy5 - SUB Vx, Vy
            // Set Vx = Vx - Vy, set VF = NOT borrow.
            // If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results stored in Vx.
            0x5 => {
                self.registers[CARRY_REG] = (self.registers[x] > self.registers[y]) as Byte;
                self.registers[x] = self.registers[x].wrapping_sub(self.registers[y]);
            },

            // 8xy6 - SHR Vx {, Vy}
            // Set Vx = Vx SHR 1.
            // If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0. Then Vx is divided by 2.
            0x6 => {
                self.registers[CARRY_REG] = self.registers[x] & 0x1;
                self.registers[x] >>= 1;
            },

            // 8xy7 - SUBN Vx, Vy
            // Set Vx = Vy - Vx, set VF = NOT borrow.
            // If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and the results stored in Vx.
            0x7 => {
                self.registers[CARRY_REG] = (self.registers[y] > self.registers[x]) as Byte;
                self.registers[x] = self.registers[y] - self.registers[x];
            },

            // 8xyE - SHL Vx {, Vy}
            // Set Vx = Vx SHL 1.
            // If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to 0. Then Vx is multiplied by 2.
            0xE => {
                self.registers[CARRY_REG] = (self.registers[x] & 0x80) >> 7;
                self.registers[x] <<= 1;
            },

            _ => eprintln!("Unknown logical op: {:#X}", self.op),
        }
    }

    /// Annn - LD I, addr
    /// Set I = nnn.
    /// The value of register I is set to nnn.
    fn set_index(&mut self) {
        self.index = self.op & 0x0FFF;
    }

    /// Cxkk - RND Vx, byte
    /// Set Vx = random byte AND kk.
    /// The interpreter generates a random number from 0 to 255,
    /// which is then ANDed with the value kk. The results are stored in Vx.
    fn rand(&mut self) {
        let reg = ((self.op & 0x0F00) >> 8) as usize;
        let val = (self.op & 0x00FF) as Byte;

        self.registers[reg] = rand::random::<Byte>() & val;
    }

    fn draw(&mut self) {
        let x = self.registers[((self.op & 0x0F00) >> 8) as usize] as u8;
        let y = self.registers[((self.op & 0x00F0) >> 4) as usize] as u8;
        let h = (self.op & 0x000F) as u8;

        self.registers[0xF] = 0;

        // self.display = [[false; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize];

        for h in 0..h {
            let line = self.memory[(self.index + h as u16) as usize];
            for w in 0..8 {
                let pixel = (line >> (7 - w)) & 0x1;
                if pixel == 1 {
                    let x = (x + w) % (SCREEN_WIDTH as u8);
                    let y = (y + h) % (SCREEN_HEIGHT as u8);
                    if self.display[y as usize][x as usize] {
                        self.registers[0xF] = 1;
                    }
                    self.display[y as usize][x as usize] ^= true;
                }
            }
        }
    }

    fn keyboard_op(&mut self) {
        println!("Keyboard op: {:#X}", self.op);

        let reg = ((self.op & 0x0F00) >> 8) as usize;

        match self.op & 0x00FF {
            // Ex9E - SKP Vx
            // Skip next instruction if key with the value of Vx is pressed.
            // Checks the keyboard, and if the key corresponding to the value of Vx is currently in the down position, PC is increased by 2.
            0x9E => {
                if self.key == self.registers[reg] {
                    self.program_counter += 2;
                }
            },

            // ExA1 - SKNP Vx
            // Skip next instruction if key with the value of Vx is not pressed.
            // Checks the keyboard, and if the key corresponding to the value of Vx is currently in the up position, PC is increased by 2.
            0xA1 => {
                if self.key != self.registers[reg] {
                    self.program_counter += 2;
                }
            },

            _ => eprintln!("Unknown keyboard op: {:#X}", self.op),
        }
    }

    fn misc_op(&mut self) {
        println!("Misc op: {:#X}", self.op);
        let reg = ((self.op & 0x0F00) >> 8) as usize;
        match self.op & 0x00FF {
            0x07 => self.registers[reg] = self.delay_timer,
            0x0A => self.registers[reg] = self.key,
            0x15 => self.delay_timer = self.registers[reg],
            0x18 => self.sound_timer = self.registers[reg],
            0x1E => self.index += self.registers[reg] as u16,
            0x29 => self.index = self.registers[reg] as u16 * 5,
            0x33 => {
                self.memory[self.index as usize] = self.registers[reg] / 100;
                self.memory[self.index as usize + 1] = (self.registers[reg] / 10) % 10;
                self.memory[self.index as usize + 2] = self.registers[reg] % 10;
            },
            0x55 => {
                for i in 0..=reg {
                    self.memory[self.index as usize + i] = self.registers[i];
                }
                self.index = self.index + reg as u16 + 1;
            },
            0x65 => {
                for i in 0..=reg {
                    self.registers[i] = self.memory[self.index as usize + i];
                }
                self.index = self.index + reg as u16 + 1;
            },
            _ => eprintln!("Unhandled misc opcode: {:#X}", self.op)
        }
    }
}

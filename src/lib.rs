use rand::random;

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_REGISTERS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;

const START_ADDRESS: u16 = 0x200;

const FONTSET_SIZE: usize = 80;
const FONTSET: [u8; FONTSET_SIZE] = [
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

pub struct Hachi {
    program_counter: u16,
    ram: [u8; RAM_SIZE],
    display: [bool; DISPLAY_WIDTH * DISPLAY_HEIGHT],
    v_registers: [u8; NUM_REGISTERS],
    i_register: u16,
    stack_pointer: u16,
    stack: [u16; STACK_SIZE],
    keys: [bool; NUM_KEYS],
    delay_timer: u8,
    sound_timer: u8,
}

impl Hachi {
    pub fn new() -> Self {
        let mut hachi = Self {
            program_counter: START_ADDRESS,
            ram: [0; RAM_SIZE],
            display: [false; DISPLAY_WIDTH * DISPLAY_HEIGHT],
            v_registers: [0; NUM_REGISTERS],
            i_register: 0,
            stack_pointer: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            delay_timer: 0,
            sound_timer: 0,
        };

        hachi.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        hachi
    }

    pub fn reset(&mut self) {
        self.program_counter = START_ADDRESS;
        self.ram = [0; RAM_SIZE];
        self.display = [false; DISPLAY_WIDTH * DISPLAY_HEIGHT];
        self.v_registers = [0; NUM_REGISTERS];
        self.i_register = 0;
        self.stack_pointer = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn tick(&mut self) {
        let op = self.fetch();

        self.execute(op);
    }

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn get_display(&self) -> &[bool] {
        &self.display
    }

    pub fn get_audio(&self) -> u8 {
        self.sound_timer
    }

    pub fn keypress(&mut self, idx: usize, pressed: bool) {
        self.keys[idx] = pressed;
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDRESS as usize;
        let end = (START_ADDRESS as usize) + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    fn execute(&mut self, op: u16) {
        let d1 = (op & 0xF000) >> 12;
        let d2 = (op & 0x0F00) >> 8;
        let d3 = (op & 0x00F0) >> 4;
        let d4 = op & 0x000F;

        // parse opcode via pattern matching

        match (d1, d2, d3, d4) {
            (0, 0, 0, 0) => (), // no-op
            (0, 0, 0xE, 0) => {
                // clear display
                self.display = [false; DISPLAY_WIDTH * DISPLAY_HEIGHT]
            }
            (0, 0, 0xE, 0xE) => {
                // return from subroutine
                let return_address = self.pop();
                self.program_counter = return_address;
            }
            (1, _, _, _) => {
                // jump program counter
                let nnn = op & 0xFFF;
                self.program_counter = nnn;
            }
            (2, _, _, _) => {
                let nnn = op & 0xFFF;
                self.push(self.program_counter);
                self.program_counter = nnn;
            }
            (3, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                if self.v_registers[x] == nn {
                    self.program_counter += 2;
                }
            }
            (4, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                if self.v_registers[x] != nn {
                    self.program_counter += 2;
                }
            }
            (5, _, _, 0) => {
                let x = d2 as usize;
                let y = d3 as usize;
                if self.v_registers[x] == self.v_registers[y] {
                    self.program_counter += 2;
                }
            }
            (6, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                self.v_registers[x] = nn;
            }
            (7, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                self.v_registers[x] = self.v_registers[x].wrapping_add(nn);
            }
            (8, _, _, 0) => {
                let x = d2 as usize;
                let y = d3 as usize;
                self.v_registers[x] = self.v_registers[y];
            }
            (8, _, _, 1) => {
                // bitwise or
                let x = d2 as usize;
                let y = d3 as usize;
                self.v_registers[x] |= self.v_registers[y];
            }
            (8, _, _, 2) => {
                // bitwise and
                let x = d2 as usize;
                let y = d3 as usize;
                self.v_registers[x] &= self.v_registers[y];
            }
            (8, _, _, 3) => {
                // bitwise xor
                let x = d2 as usize;
                let y = d3 as usize;
                self.v_registers[x] ^= self.v_registers[y];
            }
            (8, _, _, 4) => {
                let x = d2 as usize;
                let y = d3 as usize;

                let (new_vx, carry) = self.v_registers[x].overflowing_add(self.v_registers[y]);
                let new_vf = if carry { 1 } else { 0 };

                self.v_registers[x] = new_vx;
                self.v_registers[0xF] = new_vf;
            }
            (8, _, _, 5) => {
                let x = d2 as usize;
                let y = d3 as usize;

                let (new_vx, borrow) = self.v_registers[x].overflowing_sub(self.v_registers[y]);
                let new_vf = if borrow { 0 } else { 1 };

                self.v_registers[x] = new_vx;
                self.v_registers[0xF] = new_vf;
            }
            (8, _, _, 6) => {
                let x = d2 as usize;
                let least_significant_bit = self.v_registers[x] & 1;
                self.v_registers[x] >>= 1;
                self.v_registers[0xF] = least_significant_bit;
            }
            (8, _, _, 7) => {
                let x = d2 as usize;
                let y = d3 as usize;

                let (new_vx, borrow) = self.v_registers[y].overflowing_sub(self.v_registers[x]);
                let new_vf = if borrow { 0 } else { 1 };

                self.v_registers[x] = new_vx;
                self.v_registers[0xF] = new_vf;
            }
            (8, _, _, 0xE) => {
                let x = d2 as usize;
                let most_significant_bit = (self.v_registers[x] >> 7) & 1;
                self.v_registers[x] <<= 1;
                self.v_registers[0xF] = most_significant_bit;
            }
            (9, _, _, 0) => {
                let x = d2 as usize;
                let y = d3 as usize;
                if self.v_registers[x] != self.v_registers[y] {
                    self.program_counter += 2;
                }
            }
            (0xA, _, _, _) => {
                let nnn = op & 0xFFF;
                self.i_register = nnn;
            }
            (0xB, _, _, _) => {
                let nnn = op & 0xFFF;
                self.program_counter = (self.v_registers[0] as u16) + nnn;
            }
            (0xC, _, _, _) => {
                let x = d2 as usize;
                let nn = (op & 0xFF) as u8;
                let rn: u8 = random();
                self.v_registers[x] = rn & nn;
            }
            (0xD, _, _, _) => {
                let x_coord = self.v_registers[d2 as usize] as u16;
                let y_coord = self.v_registers[d3 as usize] as u16;
                let num_rows = d4;
                let mut flipped = false;

                for y_line in 0..num_rows {
                    let addr = self.i_register + y_line;
                    let pixels = self.ram[addr as usize];

                    for x_line in 0..8 {
                        if (pixels & (0b1000_0000 >> x_line)) != 0 {
                            // Sprites should wrap around screen, so apply modulo
                            let x = (x_coord + x_line) as usize % DISPLAY_WIDTH;
                            let y = (y_coord + y_line) as usize % DISPLAY_HEIGHT;
                            let idx = x + DISPLAY_WIDTH * y;
                            flipped |= self.display[idx];
                            self.display[idx] ^= true;
                        }
                    }
                }

                if flipped {
                    self.v_registers[0xF] = 1;
                } else {
                    self.v_registers[0xF] = 0;
                }
            }
            (0xE, _, 9, 0xE) => {
                let x = d2 as usize;
                let vx = self.v_registers[x];
                let key = self.keys[vx as usize];
                if key {
                    self.program_counter += 2;
                }
            }
            (0xE, _, 0xA, 1) => {
                let x = d2 as usize;
                let vx = self.v_registers[x];
                let key = self.keys[vx as usize];
                if !key {
                    self.program_counter += 2;
                }
            }
            (0xF, _, 0, 7) => {
                let x = d2 as usize;
                self.v_registers[x] = self.delay_timer;
            }
            (0xF, _, 0, 0xA) => {
                let x = d2 as usize;
                let mut pressed = false;
                for i in 00..self.keys.len() {
                    if self.keys[i] {
                        self.v_registers[x] = i as u8;
                        pressed = true;
                        break;
                    }
                }

                if !pressed {
                    self.program_counter -= 2;
                }
            }
            (0xF, _, 1, 5) => {
                let x = d2 as usize;
                self.delay_timer = self.v_registers[x];
            }
            (0xF, _, 1, 8) => {
                let x = d2 as usize;
                self.sound_timer = self.v_registers[x];
            }
            (0xF, _, 1, 0xE) => {
                let x = d2 as usize;
                let vx = self.v_registers[x] as u16;
                self.i_register = self.i_register.wrapping_add(vx);
            }
            (0xF, _, 2, 9) => {
                let x = d2 as usize;
                let c = self.v_registers[x] as u16;
                self.i_register = c * 5;
            }
            (0xF, _, 3, 3) => {
                // binary coded decimal
                let x = d2 as usize;
                let vx = self.v_registers[x] as f32;

                let hundreds = (vx / 100.0).floor() as u8;
                let tens = ((vx / 10.0) % 10.0).floor() as u8;
                let ones = (vx % 10.0) as u8;

                self.ram[self.i_register as usize] = hundreds;
                self.ram[(self.i_register + 1) as usize] = tens;
                self.ram[(self.i_register + 2) as usize] = ones;
            }
            (0xF, _, 5, 5) => {
                let x = d2 as usize;
                let i = self.i_register as usize;
                for idx in 0..=x {
                    self.ram[i + idx] = self.v_registers[idx];
                }
            }
            (0xF, _, 6, 5) => {
                let x = d2 as usize;
                let i = self.i_register as usize;
                for idx in 0..=x {
                    self.v_registers[idx] = self.ram[i + idx];
                }
            }
            (_, _, _, _) => unimplemented!("implement opcode {}", op),
        }
    }

    fn fetch(&mut self) -> u16 {
        let h_byte = self.ram[self.program_counter as usize] as u16;
        let l_byte = self.ram[(self.program_counter + 1) as usize] as u16;
        let op = (h_byte << 8) | l_byte;
        self.program_counter += 2;
        op
    }

    fn push(&mut self, val: u16) {
        self.stack[self.stack_pointer as usize] = val;
        self.stack_pointer += 1;
    }

    fn pop(&mut self) -> u16 {
        self.stack_pointer -= 1;
        self.stack[self.stack_pointer as usize]
    }
}

impl Default for Hachi {
    fn default() -> Self {
        Self::new()
    }
}

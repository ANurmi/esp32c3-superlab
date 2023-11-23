#![allow(dead_code)]
#[derive(PartialEq)]
pub struct ShiftRegister {
    reg : [u64;5],
    entries : u32,

}

impl ShiftRegister {

    pub fn new() -> Self {
        ShiftRegister { reg: [0;5], entries: 0u32 }
    }

    pub fn insert(&mut self, val: u64) {
        for idx in 1..(self.reg.len()-1) {
            self.reg[idx] = self.reg[idx-1]; 
        }

        self.reg[0] = val;

        if self.entries < self.reg.len() as u32 {
            self.entries = self.entries + 1;
        }
    }
    
    pub fn avg(&mut self) -> u64 {
        self.reg.iter().sum::<u64>()/(self.reg.len() as u64)
    }

    // report if shift register has been fully populated
    pub fn valid_entries(&self) -> bool {
        self.entries >= self.reg.len() as u32
    }

    /// Don't use this. It's for testing :-)
    fn as_array(&self) -> &[Option<u64>] {
        todo!()
    }
}

#[test]
fn insert_works() {
    let mut sr = ShiftRegister::new();

    sr.insert(1);
    assert_eq!(sr.as_array(), &[Some(1), None, None]);

    sr.insert(2);
    assert_eq!(sr.as_array(), &[Some(2), Some(1), None]);

    sr.insert(3);
    assert_eq!(sr.as_array(), &[Some(3), Some(2), Some(1)]);

    sr.insert(4);
    assert_eq!(sr.as_array(), &[Some(4), Some(3), Some(2)]);
}

fn avg_works() {
    let mut sr = ShiftRegister::new();

    sr.insert(1);
    sr.insert(2);
    sr.insert(3);

    assert_eq!(sr.avg(), 1);

    sr.insert(4);

    assert_eq!(sr.avg(), 1);
}

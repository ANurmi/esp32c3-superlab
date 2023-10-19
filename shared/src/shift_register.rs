#[derive(PartialEq)]
pub struct ShiftRegister {
    pub reg : [u64;3],
}

impl ShiftRegister {

    pub fn insert(&mut self, val: u64) {
        for idx in 1..2 {
            self.reg[idx] = self.reg[idx-1]; 
        }
        self.reg[0] = val
    }
    pub fn avg(&mut self) -> u64 {
        self.reg.iter().sum::<u64>()/3
    }

    /// Don't use this. It's for testing :-)
    fn as_array(&self) -> &[Option<u64>] {
        todo!()
    }
}

#[test]
fn insert_works() {
    let mut sr = ShiftRegister{reg:[0;3]};

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
    let mut sr = ShiftRegister{reg:[0;3]};

    sr.insert(1);
    sr.insert(2);
    sr.insert(3);

    assert_eq!(sr.avg(), 1);

    sr.insert(4);

    assert_eq!(sr.avg(), 1);
}

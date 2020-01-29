#[derive(Debug)]
pub enum Branch {
    Taken,
    NotTaken,
}

#[derive(Debug)]
pub enum Predictor {
    StronglyNotTaken,
    NotTaken,
    Taken,
    StronglyTaken,
}

impl Predictor {
    pub fn new() -> Self {
        Self::NotTaken {}
    }

    pub fn transition(self, tran: Branch) -> Self {
        match tran {
            Branch::Taken => match self {
                Self::StronglyNotTaken => Self::NotTaken,
                Self::NotTaken => Self::Taken {},
                Self::Taken => Self::StronglyTaken {},
                Self::StronglyTaken => self,
            },

            Branch::NotTaken => match self {
                Self::StronglyNotTaken => self,
                Self::NotTaken => Self::StronglyNotTaken {},
                Self::Taken => Self::NotTaken {},
                Self::StronglyTaken => Self::Taken {},
            },
        }
    }
}

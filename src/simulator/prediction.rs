#[derive(Debug)]
pub enum Branch {
    Taken(u16),
    NotTaken,
    Jump(u16),
    None,
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

    pub fn predicts_branch(&self) -> bool {
        match *self {
            Self::StronglyNotTaken | Self::NotTaken => false,
            Self::Taken | Self::StronglyTaken => true,
        }
    }

    pub fn transition(self, tran: Branch) -> Self {
        match tran {
            Branch::Jump | Branch::None => self,
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

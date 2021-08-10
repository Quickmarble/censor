use std::cmp::Ordering;

pub trait Clip {
    fn clip(self, min: Self, max: Self) -> Self;
}

impl<T> Clip for T where T: Copy + std::cmp::PartialOrd {
    fn clip(self, min: Self, max: Self) -> Self {
        if self < min { return min; }
        if self > max { return max; }
        return self;
    }
}

pub trait CyclicClip {
    fn cyclic_clip(self, period: Self) -> Self;
}
impl<T> CyclicClip for T
        where T: 
            Copy + Default + std::cmp::PartialOrd
            + std::ops::Add<Output = T> + std::ops::Sub<Output = T> {
    fn cyclic_clip(self, period: Self) -> Self {
        let zero = Self::default();
        if self < zero {
            let mut t = self;
            while t < zero {
                t = t + period;
            }
            return t;
        }
        if self >= period {
            let mut t = self;
            while t >= period {
                t = t - period;
            }
            return t;
        }
        return self;
    }
}

pub trait Lerp {
    fn lerp(self, another: Self, a: f32) -> Self;
}
impl<T> Lerp for T
        where T: Copy + std::cmp::PartialOrd
        + std::ops::Add<Output=T> + std::ops::Mul<f32, Output=T> {
    fn lerp(self, another: Self, a: f32) -> Self {
        return self * (1.-a) + another * a;
    }
}

#[derive(Copy, Clone, PartialOrd)]
pub struct PackedF32(pub f32);
impl std::hash::Hash for PackedF32 {
    fn hash<H>(&self, state: &mut H) where H: std::hash::Hasher {
        self.0.to_bits().hash(state)
    }
}
impl PartialEq for PackedF32 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}
impl Eq for PackedF32 {}
impl Ord for PackedF32 {
    fn cmp(&self, other: &Self) -> Ordering {
        if self < other {
            return Ordering::Less;
        }
        if self > other {
            return Ordering::Greater;
        }
        return self.0.to_bits().cmp(&other.0.to_bits());
    }
}

pub fn abs_diff<T: std::ops::Sub<Output=T>+PartialOrd+Copy>(x: T, y: T) -> T {
    if x < y {
        y - x
    } else {
        x - y
    }
}

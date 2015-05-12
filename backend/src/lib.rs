extern crate rustc_serialize;

use std::cell::Cell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::io::prelude::*;
use std::path::Path;
use std::slice::Iter;
use rustc_serialize::json::{decode, encode};

pub type Id = u8;
pub type Concentration = f32;
pub type ChemicalMap = HashMap<Id, Chemical>;
pub type DeltaMap = HashMap<Id, Concentration>;


#[derive(RustcEncodable, RustcDecodable)]
pub struct Chemical {
    id: Id,
    concentration: Concentration,
}

impl Chemical {
    pub fn new(id: Id) -> Chemical {
        Chemical { id: id, concentration: 0.0 }
    }

    pub fn with_concentration(id: Id, concentration: Concentration) -> Chemical {
        Chemical { id: id, concentration: concentration }
    }
}


#[derive(RustcEncodable, RustcDecodable)]
pub struct Emitter {
    chemical: Id,
    gain: f32,
}

impl Emitter {
    pub fn new(chemical: Id, gain: f32) -> Emitter {
        Emitter { chemical: chemical, gain: gain }
    }

    pub fn step(&self, deltas: &mut DeltaMap) {
        let val = deltas.entry(self.chemical).or_insert(0.0);
        *val += self.gain;
        if *val > 1.0 { *val = 1.0 }
    }
}

#[derive(RustcEncodable, RustcDecodable)]
pub enum ReactionType {
    /// A + B -> C + D
    Normal(Chemical, Chemical, Chemical, Chemical),
    /// A + B -> C
    Fusion(Chemical, Chemical, Chemical),
    /// A -> nothing
    Decay(Chemical),
    /// A + B -> A + C
    Catalytic(Chemical, Chemical, Chemical),
    /// A + B -> A
    CatalyticBreakdown(Chemical, Chemical),
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct Reaction {
    kind: ReactionType,
    rate: u8,
    tick: Cell<u8>,
}

impl Reaction {
    pub fn new(kind: ReactionType, rate: u8) -> Reaction {
        Reaction { kind: kind, rate: rate, tick: Cell::new(0) }
    }

    pub fn step(&self, map: &ChemicalMap, deltas: &mut DeltaMap) {
        self.tick.set(self.tick.get() + 1);
        if self.tick.get() < self.rate { return }
        self.tick.set(0);
        match self.kind {
            ReactionType::Normal(ref a, ref b, ref c, ref d) => {
                let n = (map[&a.id].concentration / a.concentration)
                        .min(map[&b.id].concentration / b.concentration); 
                let mut update = |c: &Chemical, add: bool| {
                    let val = deltas.entry(c.id).or_insert(0.0);
                    if add {
                        *val += n * c.concentration
                    } else {
                        *val -= n * c.concentration
                    }
                    if *val > 1.0 { *val = 1.0 } else if *val < 0.0 { *val = 0.0 }
                };
                update(a, false);
                update(b, false);
                update(c, true);
                update(d, true);
            },
            ReactionType::Fusion(ref a, ref b, ref c) => {
                let n = (map[&a.id].concentration / a.concentration)
                        .min(map[&b.id].concentration / b.concentration); 
                let mut update = |c: &Chemical, add: bool| {
                    let val = deltas.entry(c.id).or_insert(0.0);
                    if add {
                        *val += n * c.concentration
                    } else {
                        *val -= n * c.concentration
                    }
                    if *val > 1.0 { *val = 1.0 } else if *val < 0.0 { *val = 0.0 }
                };                
                update(a, false);
                update(b, false);
                update(c, true);
            },
            ReactionType::Decay(ref a) => {
                let n = map[&a.id].concentration / a.concentration;
                let val = deltas.entry(a.id).or_insert(0.0);
                *val -= n * a.concentration;
                if *val > 1.0 { *val = 1.0 } else if *val < 0.0 { *val = 0.0 }
            },
            ReactionType::Catalytic(ref a, ref b, ref c) => {
                let n = (map[&a.id].concentration / a.concentration)
                        .min(map[&b.id].concentration / b.concentration); 
                let mut update = |c: &Chemical, add: bool| {
                    let val = deltas.entry(c.id).or_insert(0.0);
                    if add {
                        *val += n * c.concentration
                    } else {
                        *val -= n * c.concentration
                    }
                    if *val > 1.0 { *val = 1.0 } else if *val < 0.0 { *val = 0.0 }
                };
                update(b, false);
                update(c, true);
            },
            ReactionType::CatalyticBreakdown(ref a, ref b) => {
                let n = (map[&a.id].concentration / a.concentration)
                        .min(map[&b.id].concentration / b.concentration); 
                let val = deltas.entry(b.id).or_insert(0.0);
                *val -= n * b.concentration;
                if *val > 1.0 { *val = 1.0 } else if *val < 0.0 { *val = 0.0 }
            },
        }
    }
}

#[derive(RustcEncodable, RustcDecodable)]
pub enum ReceptorType {
    /// Receptor triggers when concentration is below threshold.
    LowerBound,
    /// Receptor triggers when concentration is above threshold.
    UpperBound,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct Receptor {
    kind: ReceptorType,
    chemical: Id,
    gain: f32,
    threshold: f32,
}

impl Receptor {
    pub fn new(kind: ReceptorType, chemical: Id, gain: f32, threshold: f32) -> Receptor {
        Receptor { kind: kind, chemical: chemical, gain: gain, threshold: threshold }
    }

    pub fn step(&self, map: &ChemicalMap, deltas: &DeltaMap) -> Option<f32> {
        let prev = map[&self.chemical].concentration;
        let curr = prev - deltas.get(&self.chemical).map(|u| *u).unwrap_or(0.0);
        match self.kind {
            ReceptorType::LowerBound => if prev > self.threshold && curr < self.threshold {
                Some(curr * self.gain)
            } else {
                None   
            },
            ReceptorType::UpperBound => if prev < self.threshold && curr > self.threshold {
                Some(curr * self.gain)
            } else {
                None   
            },
        }
    }
}

#[derive(RustcEncodable, RustcDecodable)]
pub enum Gene {
    Emitter(Emitter),
    Reaction(Reaction),
    Receptor(Receptor),
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct Genome {
    genes: Vec<Gene>
}

impl Genome {
    pub fn new(genes: Vec<Gene>) -> Genome {
        Genome { genes: genes }
    }

    pub fn load(path: &Path) -> Result<Genome> {
        let mut f = try!(File::open(path));
        let mut data = String::new();
        try!(f.read_to_string(&mut data));
        decode(&data).map_err(|_|
            Error::new(ErrorKind::InvalidInput, "Failed to decode genome.")
        )
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let mut f = try!(File::create(path));
        try!(f.write_all(try!(encode(self).map_err(|_|
            Error::new(ErrorKind::InvalidInput, "Failed to encode genome.")
        )).as_bytes()));
        f.flush()
    }

    pub fn iter(&self) -> Iter<Gene> {
        self.genes.iter()
    }
}

extern crate backend;

use backend::*;

fn main() {
    let mut genome = Genome::new();
    let mut creature = Creature::new();
    genome.init(&mut creature);
    let mut steps = 0;
    while steps < 300 || creature.age() == Age::Baby {
        while steps < 300 {
            genome.step(&mut creature);
            steps += 1;
        }
        genome.mutate();
        steps = 0;
    }
    genome.save("evolved.json").unwrap();
}

use crate::domain;
use crate::point;
use domain::Domain;
use point::CardinalDir::*;
use std::iter::zip;

// type Texture = macroquad::texture::Texture2D;
use macroquad::texture::Image;

#[derive(Debug, Clone)]
pub struct SimplePattern {
    pub id: u8,
    pub name: String,
    pub allowed_neighbors: [(point::CardinalDir, [bool; 5]); 4],
    pub img: Image,
}

fn ora(a: [bool; 5], b: [bool; 5]) -> [bool; 5] {
    let mut c: [bool; 5] = [false; 5];
    for i in 0..5 {
        c[i] = a[i] | b[i];
    }
    return c;
}

const BI: [bool; 5] = [true, false, false, false, false];
const DLI: [bool; 5] = [false, true, false, false, false];
const LUI: [bool; 5] = [false, false, true, false, false];
const RDI: [bool; 5] = [false, false, false, true, false];
const URI: [bool; 5] = [false, false, false, false, true];
const ALLI: [bool; 5] = [true, true, true, true, true];

pub fn get_simple_patterns() -> [SimplePattern; 5] {
    let li = ora(LUI, DLI);
    let ri = ora(RDI, URI);
    let ui = ora(LUI, URI);
    let di = ora(RDI, DLI);

    let blank: SimplePattern = SimplePattern {
        id: 0,
        name: "blank".to_string(),
        allowed_neighbors: [
            (LEFT, li),  //left
            (UP, ui),    //up
            (RIGHT, ri), //right
            (DOWN, di),  // down
        ],
        img: Image {
            width: 4,
            height: 4,
            bytes: vec![
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            ],
        },
    };

    let dl: SimplePattern = SimplePattern {
        id: 1,
        name: "dl".to_string(),
        allowed_neighbors: [
            (LEFT, ri),            //left
            (UP, ora(ui, BI)),    //up
            (RIGHT, ora(ri, BI)), //right
            (DOWN, ui),            // down
        ],
        img: Image {
            width: 4,
            height: 4,
            bytes: vec![
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0,
                0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0,
                255, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0,
                255, 255, 255, 255, 255,
            ],
        },
    };

    let lu: SimplePattern = SimplePattern {
        id: 2,
        name: "lu".to_string(),
        allowed_neighbors: [
            (LEFT, ri),            //left
            (UP, di),              //up
            (RIGHT, ora(ri, BI)), //right
            (DOWN, ora(di, BI)),  // down
        ],
        img: Image {
            width: 4,
            height: 4,
            bytes: vec![
                255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255,
                0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0,
                0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255,
            ],
        },
    };

    let rd: SimplePattern = SimplePattern {
        id: 3,
        name: "rd".to_string(),
        allowed_neighbors: [
            (LEFT, ora(li, BI)), //left
            (UP, ora(ui, BI)),   //up
            (RIGHT, li),          //right
            (DOWN, ui),           // down
        ],
        img: Image {
            width: 4,
            height: 4,
            bytes: vec![
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255,
                0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0,
                0, 255, 255, 255, 255, 255,
            ],
        },
    };

    let ur: SimplePattern = SimplePattern {
        id: 4,
        name: "ur".to_string(),
        allowed_neighbors: [
            (LEFT, ora(li, BI)), //left
            (UP, di),             //up
            (RIGHT, li),          //right
            (DOWN, ora(di, BI)), //down
        ],
        img: Image {
            width: 4,
            height: 4,
            bytes: vec![
                255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0,
                0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255,
            ],
        },
    };
    let pats = [blank, dl, lu, rd, ur];
    return pats;
}

#[cfg(test)]
mod test {
    use super::ora;

    #[test]
    fn ora_all_true() {
        let a = [true, true, true, true, true];
        let b = [true, true, true, true, true];
        assert_eq!(ora(a , b), [true, true, true, true, true]);
        assert_eq!(b, [true, true, true, true, true]);
        assert_eq!(a, [true, true, true, true, true]);
    }
    #[test]
    fn ora_alternating_is_all_true() {
        let a = [false, true, false, true, false];
        let b = [true, false, true, false, true];
        assert_eq!(ora(a , b), [true, true, true, true, true]);
        assert_eq!(a, [false, true, false, true, false]);
        assert_eq!(b, [true, false, true, false, true]);
    }
}

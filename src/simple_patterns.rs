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
    for i in (0..5) {
        c[i] = a[i] | b[i];
    }
    return c;
}

const BLANK: [bool; 5] = [true, false, false, false, false];
const DOWN_LEFT: [bool; 5] = [false, true, false, false, false];
const LEFT_UP: [bool; 5] = [false, false, true, false, false];
const RIGHT_DOWN: [bool; 5] = [false, false, false, true, false];
const UP_RIGHT: [bool; 5] = [false, false, false, false, true];
const ALL: [bool; 5] = [true, true, true, true, true];
// const BLANK: [bool; 5] = [true, true, true, true, true];
// const DOWN_LEFT: [bool; 5] = [true, true, true, true, true];
// const LEFT_UP: [bool; 5] = [true, true, true, true, true];
// const RIGHT_DOWN: [bool; 5] = [true, true, true, true, true];
// const UP_RIGHT: [bool; 5] = [true, true, true, true, true];
// const ALL: [bool; 5] = [true, true, true, true, true];


pub fn get_simple_patterns() -> [SimplePattern; 5] {

    let has_left: [bool; 5] = ora(LEFT_UP, DOWN_LEFT);
    let has_right: [bool; 5] = ora(RIGHT_DOWN, UP_RIGHT);
    let has_up: [bool; 5] = ora(LEFT_UP, UP_RIGHT);
    let has_down: [bool; 5] = ora(RIGHT_DOWN, DOWN_LEFT);

    let blank_on_left: [bool; 5] = ora(has_left, BLANK);
    let blank_on_up: [bool; 5] = ora(has_up, BLANK);
    let blank_on_right: [bool; 5] = ora(has_right, BLANK);
    let blank_on_down: [bool; 5] = ora(has_down, BLANK);

    let blank: SimplePattern = SimplePattern {
        id: 0,
        name: "blank".to_string(),
        allowed_neighbors: [
            (LEFT, blank_on_right),  //left
            (UP, blank_on_down),    //up
            (RIGHT, blank_on_left), //right
            (DOWN, blank_on_up),  // down
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
            (LEFT, has_right),            //left
            (UP, blank_on_down),    //up
            (RIGHT, blank_on_left), //right
            (DOWN, has_up),            // down
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
            (LEFT, has_right),            //left
            (UP, has_down),              //up
            (RIGHT, blank_on_left), //right
            (DOWN, blank_on_up),  // down
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
            (LEFT, blank_on_right), //left
            (UP, blank_on_down),   //up
            (RIGHT, has_left),          //right
            (DOWN, has_up),           // down
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
            (LEFT, blank_on_right), //left
            (UP, has_down),             //up
            (RIGHT, has_left),          //right
            (DOWN, blank_on_up), //down
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
    }

    #[test]
    fn ora_all_false() {
        let a = [false, false, false, false, false];
        let b = [false, false, false, false, false];
        assert_eq!(ora(a , b), [false, false, false, false, false]);
    }

    #[test]
    fn ora_one_true() {
        let a = [true, false, false, false, false];
        let b = [false, false, false, false, false];
        assert_eq!(ora(a , b), [true, false, false, false, false]);
    }

    #[test]
    fn ora_alternating_is_all_true() {
        let a = [false, true, false, true, false];
        let b = [true, false, true, false, true];
        assert_eq!(ora(a , b), [true, true, true, true, true]);
    }
}

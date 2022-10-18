type PointTup = (i32, i32);
type IdVec = Vec<u8>;
type Texture = macroquad::texture::Texture2D;
use macroquad::texture::Image;

#[derive(Debug, Clone)]
pub struct SimplePattern {
    pub id: u8,
    pub name: String,
    pub allowed_neighbors: [(PointTup, IdVec); 4],
    pub img: Image,
}

fn andv(a: &[u8], b: &[u8]) -> Vec<u8> {
    return a.iter().zip(b.iter()).map(|(x, y)| x * y).collect();
}

pub fn get_simple_patterns() -> [SimplePattern; 5] {
    let bi: &[u8] = &[1, 0, 0, 0, 0];
    let dli: &[u8] = &[0, 1, 0, 0, 0];
    let lui: &[u8] = &[0, 0, 1, 0, 0];
    let rdi: &[u8] = &[0, 0, 0, 1, 0];
    let uri: &[u8] = &[0, 0, 0, 0, 1];
    let _alli: &[u8] = &[1, 1, 1, 1, 1];

    let li: &[u8] = &andv(lui, dli);
    let ri: &[u8] = &andv(rdi, uri);
    let ui: &[u8] = &andv(lui, uri);
    let di: &[u8] = &andv(rdi, dli);

    let blank: SimplePattern = SimplePattern {
        id: 0,
        name: "blank".to_string(),
        allowed_neighbors: [
            ((-1, 0), li.to_vec()), //left
            ((0, 1), ui.to_vec()),  //up
            ((1, 0), ri.to_vec()),  //right
            ((0, -1), di.to_vec()), // down
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
            ((-1, 0), ri.to_vec()), //left
            ((0, 1), andv(ui, bi)), //up
            ((1, 0), andv(ri, bi)), //right
            ((0, -1), ui.to_vec()), // down
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
            ((-1, 0), ri.to_vec()),  //left
            ((0, 1), di.to_vec()),   //up
            ((1, 0), andv(ri, bi)),  //right
            ((0, -1), andv(di, bi)), // down
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
            ((-1, 0), andv(li, bi)), //left
            ((0, 1), andv(ui, bi)),  //up
            ((1, 0), li.to_vec()),   //right
            ((0, -1), ui.to_vec()),  // down
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
            ((-1, 0), andv(li, bi)), //left
            ((0, 1), di.to_vec()),   //up
            ((1, 0), li.to_vec()),   //right
            ((0, -1), andv(di, bi)), //down
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

use crossterm::{
    cursor,
    event::poll,
    event::Event,
    event::KeyCode,
    execute,
    style::{self, Stylize},
    terminal,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand, QueueableCommand,
};
use rand::Rng;
use std::fs::File;
use std::path::Path;
use std::{
    io::{self, Write},
    task::Poll,
    time::Duration,
};
use thiserror::Error;
struct Dungeon {
    tree: DungeonTree,
    homogeneity: f64, //Will be clamped between 0 and 1. Increases the variance for the splitting of sub-dungeons.
    splits: i64, //How many times the sub-dungeons will be split. Going to high with too small of a dungeon can produce odd results.
    split_direction: split_dirs, // Split the sub-dungeons horziontally or vertically.
}

enum split_dirs {
    ALWAYS_VERT,
    ALWAYS_HORIZONTAL,
    RANDOM,
}

#[derive(Debug, Error)]
pub enum TreeError {
    #[error("Tree already has root node... ")]
    RootErr,

    #[error("Invalid index for tree...")]
    IndexError,

    #[error("Can't split into sub-dungeons from dungeon of length or width smaller than three...")]
    SubDungeonSplitError,
}

#[derive(Clone, Debug)]
pub struct DungeonNode {
    //x1, y1, x2, y2
    coords: Option<(i32, i32, i32, i32)>,
    left: Option<usize>,
    right: Option<usize>,
    room: Option<(i32, i32, i32, i32)>,
}

pub struct DungeonTree {
    nodes: Vec<DungeonNode>,
}

impl DungeonTree {
    //Return a empty tree with no nodes
    pub fn new(splits: usize) -> DungeonTree {
        DungeonTree {
            nodes: Vec::with_capacity(splits * 2),
        }
    }

    //Sets the root of the tree
    pub fn setRoot(&mut self, root_node: DungeonNode) -> Result<(), TreeError> {
        if self.nodes.len() != 0 {
            Err(TreeError::RootErr)
        } else {
            *self = DungeonTree {
                nodes: vec![root_node; 1],
            };
            Ok(())
        }
    }

    pub fn build_rooms(&mut self, offsets: (i32, i32, i32, i32)) -> Result<(), TreeError> {
        let min_x_offset = offsets.0;
        let min_y_offset = offsets.1;
        let max_x_offset = offsets.2;
        let max_y_offset = offsets.3;

        for sub_dungeons in self.nodes.iter_mut().enumerate() {
            if (sub_dungeons.0 == 0) {
                sub_dungeons.1.room = None;
                continue;
            }

            let sub_width = sub_dungeons.1.coords.unwrap().2 - sub_dungeons.1.coords.unwrap().0;
            let sub_height = sub_dungeons.1.coords.unwrap().3 - sub_dungeons.1.coords.unwrap().1;

            if (sub_width <= 3 || sub_height <= 3) {
                println!("Sub dungeon is too small!");
                sub_dungeons.1.room = None;
                continue;
            }

            //Simplistic, randomize later
            let dims = (
                sub_dungeons.1.coords.unwrap().0 + min_x_offset,
                sub_dungeons.1.coords.unwrap().1 + min_y_offset,
                sub_dungeons.1.coords.unwrap().2 - max_x_offset,
                sub_dungeons.1.coords.unwrap().3 - max_y_offset,
            );

            sub_dungeons.1.room = Some(dims);
        }
        Ok(())
    }

    //At the given node, split it into two sub-dungeons. If sub-dungeons already exist at the child node locations, they will be over-written.
    //Be careful with this, as your DungeonTree node vector will continue to increase in size even if it isn't necessary.
    pub fn split_sub_dungeon(&mut self, vert: bool, node_idx: i32) -> Result<(), TreeError> {
        let mut split_pos: i32;
        let mut split_range: (i32, i32);
        let root_idx: usize;
        let root_node: DungeonNode;

        match self
            .nodes
            .iter_mut()
            .enumerate()
            .find(|c| c.0 == node_idx as usize)
        {
            Some((idx, node)) => {
                root_node = node.clone();
                root_idx = idx;
                node.left = Some(2 * idx + 1);
                node.right = Some(2 * idx + 2);
                if vert {
                    split_range = (node.coords.unwrap().0, node.coords.unwrap().2)
                } else {
                    split_range = (node.coords.unwrap().1, node.coords.unwrap().3)
                }
            }
            None => return Err(TreeError::IndexError),
        }

        //Check later for balance
        split_pos = (split_range.0 + split_range.1) / 2;
        split_pos = (split_pos as f32 * rand::thread_rng().gen_range(0.35..0.75)) as i32;

        self.nodes.resize(self.nodes.len() + 2, root_node.clone());
        if vert {
            self.nodes[2 * root_idx + 1] = DungeonNode {
                coords: Some((
                    root_node.coords.unwrap().0 + 1,
                    root_node.coords.unwrap().1 + 1,
                    split_pos,
                    root_node.coords.unwrap().3 - 1,
                )),
                left: None,
                right: None,
                room: None,
            };
            self.nodes[2 * root_idx + 2] = DungeonNode {
                coords: Some((
                    split_pos + 1,
                    root_node.coords.unwrap().1 + 1,
                    root_node.coords.unwrap().2 - 1,
                    root_node.coords.unwrap().3 - 1,
                )),
                left: None,
                right: None,
                room: None,
            };
        } else {
            self.nodes[2 * root_idx + 1] = DungeonNode {
                coords: Some((
                    root_node.coords.unwrap().0 + 1,
                    root_node.coords.unwrap().1 + 1,
                    root_node.coords.unwrap().2 - 1,
                    split_pos,
                )),
                left: None,
                right: None,
                room: None,
            };
            self.nodes[2 * root_idx + 2] = DungeonNode {
                coords: Some((
                    root_node.coords.unwrap().0 + 1,
                    split_pos + 1,
                    root_node.coords.unwrap().2 - 1,
                    root_node.coords.unwrap().3 - 1,
                )),
                left: None,
                right: None,
                room: None,
            };
        }
        Ok(())
    }

    pub fn draw_to_file(&mut self) {

        let width = self.nodes[0].coords.unwrap().2 - self.nodes[0].coords.unwrap().0;
        let height = self.nodes[0].coords.unwrap().3 - self.nodes[0].coords.unwrap().1;

        let path = Path::new("dung.out");
        let display = path.display();
        let mut grid:Vec<Vec<char>> = vec![vec![' '; height as usize ]; width as usize];
        let mut buf = String::new();

        let mut file = match File::create(&path) {
            Err(why) => panic!("couldn't create {}: {}", display, why),
            Ok(file) => file,
        };

        for sub_dungeon in self.nodes.iter().enumerate() {



            let x1 = sub_dungeon.1.coords.unwrap().0;
            let y1 = sub_dungeon.1.coords.unwrap().1;
            let x2 = sub_dungeon.1.coords.unwrap().2;
            let y2 = sub_dungeon.1.coords.unwrap().3;

            
                for y in y1..y2 {
                    for x in x1..x2 {
                        if (y == y1 || y == y2 - 1) || (x == x1|| x == x2 - 1)
                        {
                            grid[y as usize][x as usize] = '*';
                        } else {
                            grid[y as usize][x as usize] = ' ';
                        }
                    }
                    
                }
            

            

            // let midX = (sub_dungeon.coords.unwrap().0 + sub_dungeon.coords.unwrap().2) / 2;
            // let midY = (sub_dungeon.coords.unwrap().1 + sub_dungeon.coords.unwrap().3) / 2;
            //Print node name
            // let _ = stdout
            //                 .queue(cursor::MoveTo(midX.try_into().unwrap(),midY.try_into().unwrap())).unwrap()
            //                 .queue(style::Print(sub_dung_lbl));
        }

        let rooms: Vec<_> = self.nodes.iter_mut().map(|c| c.room).collect();

        for room in rooms
        {

           if room == None {
            continue;
           }

           let room_x1 = room.unwrap().0;
           let room_y1 = room.unwrap().1;
           let room_x2 = room.unwrap().2;
           let room_y2 = room.unwrap().3;


           for y in room_y1..room_y2{
            for x in room_x1..room_x2
            {
                grid[y as usize][x as usize] = '.'; 
            }
           }

        }


        for line in grid
        {
            for c in line
            {
                buf.push_str(String::from(c).as_str());
            }
            buf.push('\n');
        }

        file.write(buf.as_str().as_bytes());
    }

    pub fn draw_sub_dungeons(&self) {
        let mut stdout = io::stdout();

        let colors = [
            "█".magenta(),
            "█".red(),
            "█".blue(),
            "█".white(),
            "█".green(),
            "█".yellow(),
        ];

        //Skip drawing the base
        let mut cpy = self.nodes.clone();
        //cpy.remove(0);

        stdout
            .execute(terminal::Clear(terminal::ClearType::All))
            .unwrap();

        let mut sub_dung_lbl = 0;
        for sub_dungeon in cpy {
            let colr = sub_dung_lbl % colors.len();

            for y in sub_dungeon.coords.unwrap().1..=sub_dungeon.coords.unwrap().3 {
                for x in sub_dungeon.coords.unwrap().0..=sub_dungeon.coords.unwrap().2 {
                    if (y == sub_dungeon.coords.unwrap().1 || y == sub_dungeon.coords.unwrap().3)
                        || (x == sub_dungeon.coords.unwrap().0
                            || x == sub_dungeon.coords.unwrap().2)
                    {
                        let _ = stdout
                            .queue(cursor::MoveTo(x.try_into().unwrap(), y.try_into().unwrap()))
                            .unwrap()
                            .queue(style::PrintStyledContent(colors[colr]));
                    }
                }
            }

            let midX = (sub_dungeon.coords.unwrap().0 + sub_dungeon.coords.unwrap().2) / 2;
            let midY = (sub_dungeon.coords.unwrap().1 + sub_dungeon.coords.unwrap().3) / 2;
            //Print node name
            // let _ = stdout
            //                 .queue(cursor::MoveTo(midX.try_into().unwrap(),midY.try_into().unwrap())).unwrap()
            //                 .queue(style::Print(sub_dung_lbl));
            sub_dung_lbl += 1;
        }
        stdout.flush().unwrap();
    }

    pub fn draw_rooms(&self) {
        let mut stdout = io::stdout();

        let colors = [
            "█".magenta(),
            "█".red(),
            "█".blue(),
            "█".white(),
            "█".green(),
            "█".yellow(),
        ];

        //Skip drawing the base
        let mut cpy = self.nodes.clone();
        //cpy.remove(0);

        //stdout.execute(terminal::Clear(terminal::ClearType::All)).unwrap();

        let mut sub_dung_lbl = 0;
        for sub_dungeon in cpy {
            let colr = sub_dung_lbl % colors.len();

            if (sub_dungeon.room == None) {
                continue;
            }

            for y in sub_dungeon.room.unwrap().1..=sub_dungeon.room.unwrap().3 {
                for x in sub_dungeon.room.unwrap().0..=sub_dungeon.room.unwrap().2 {
                    let _ = stdout
                        .queue(cursor::MoveTo(x.try_into().unwrap(), y.try_into().unwrap()))
                        .unwrap()
                        .queue(style::PrintStyledContent(colors[colr]));
                }
            }

            let midX = (sub_dungeon.room.unwrap().0 + sub_dungeon.room.unwrap().2) / 2;
            let midY = (sub_dungeon.room.unwrap().1 + sub_dungeon.room.unwrap().3) / 2;
            //Print node name
            let _ = stdout
                .queue(cursor::MoveTo(
                    midX.try_into().unwrap(),
                    midY.try_into().unwrap(),
                ))
                .unwrap()
                .queue(style::Print(sub_dung_lbl));
            sub_dung_lbl += 1;
        }
        stdout.flush().unwrap();
    }
}

fn main() {
    execute!(io::stdout(), EnterAlternateScreen);
    let mut test = DungeonTree::new(4);

    let rt: DungeonNode = DungeonNode {
        coords: Some((0, 0, 128, 128)),
        left: None,
        right: None,
        room: None,
    };

    test.setRoot(rt);

    test.split_sub_dungeon(true, 0);
    test.split_sub_dungeon(false, 1);
    //test.split_sub_dungeon(false, 2);
    test.build_rooms((1,1,1,1));
    test.draw_to_file(); 
   
    //    for node in test.nodes.iter()
    //    {
    //     println!("{:?}", node.coords);
    //    }

    //    loop {
    //     if poll(Duration::from_millis(100)).unwrap() {
    //         // It's guaranteed that `read` won't block, because `poll` returned
    //         // `Ok(true)`.
    //          let ev: crossterm::event::Event = crossterm::event::read().unwrap();
    //          let mut tt: crossterm::event::KeyCode = crossterm::event::KeyCode::Enter;

    //          match ev
    //          {
    //             Event::Key(key) => tt = key.code,
    //             _ => (),
    //          }

    //          if(tt == crossterm::event::KeyCode::Char('c'))
    //          {
    //             break;
    //          }

    //     } else {
    //         // Timeout expired, no `Event` is available
    //     }
    // }

    //    execute!(io::stdout(), LeaveAlternateScreen).unwrap();
}

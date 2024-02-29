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
use ptree::*;
use rand::Rng;
use std::{borrow::Cow, fs::File};
use std::path::Path;
use std::fmt::Display;
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

#[derive(Clone, Debug, Copy, PartialEq)]
pub struct DungeonNode {
    //x1, y1, x2, y2
    coords: Option<(i32, i32, i32, i32)>,
    node_id: usize,
    left: Option<usize>,
    right: Option<usize>,
    room: Option<(i32, i32, i32, i32)>,
}
#[derive(Clone, Debug, PartialEq)]
struct DungeonNodeRef<'a> {
    tree: &'a DungeonTree,
    node_id: usize
}

#[derive(Clone, Debug, PartialEq)]
pub struct DungeonTree {
    nodes: Vec<Option<DungeonNode>>,
}

// impl TreeItem for DungeonNode
// {
//     type Child = Self;

//     fn write_self<W: io::Write>(&self, f: &mut W, style: &Style) -> io::Result<()> {
//         write!(f, "{:?}", self.coords)
//     }

//     fn children(&self) -> Cow<[Self::Child]> {
        
//     }
// }

impl<'a> TreeItem for DungeonNodeRef<'a> {
    type Child = Self;

    fn write_self<W: io::Write>(&self, f: &mut W, style: &Style) -> io::Result<()> {
        write!(f, "{}", style.paint(self))
    }

    fn children(&self) -> Cow<[Self::Child]> {
        todo!()
    }
    // stuff here
}

impl<'a> std::fmt::Display for DungeonNodeRef<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        if(self.tree.nodes.len() > 0)
        {
            write!(fmt, "Coords: {:?}", self.tree.nodes[0])
        }
        else {
            write!(fmt, "None")
        }
    }
}

impl TreeItem for DungeonTree {
    type Child = Self;
    fn write_self<W: io::Write>(&self, f: &mut W, style: &Style) -> io::Result<()> {
        write!(f, "{}", style.paint(self))
    }
    fn children(&self) -> Cow<[Self::Child]> {

        let left_child_tree = self.get_subtree(0, true);
        let right_child_tree = self.get_subtree(0, false);
        
        //println!("{:?}", left_child_tree);
        //println!("{:?}", right_child_tree);

        if  (left_child_tree == None) && (right_child_tree != None) 
        {
            Cow::Owned(vec![right_child_tree.unwrap()])
        }

        else if (left_child_tree != None) && (right_child_tree == None) 
        {
            Cow::Owned(vec![left_child_tree.unwrap()])
        }

        else if (left_child_tree == None) && (right_child_tree == None)   
        {
            Cow::Owned(vec![])
        }

        else {
            Cow::Owned(vec![left_child_tree.unwrap(), right_child_tree.unwrap()])
        }

        
    }


}

impl std::fmt::Display for DungeonTree {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        if(self.nodes.len() > 0)
        {
            write!(fmt, "Coords: {:?}", self.nodes[0])
        }
        else {
            write!(fmt, "None")
        }
    }
}

impl DungeonNode{
    pub fn new() -> DungeonNode
    {
        DungeonNode{ coords: None, left: None, right: None, room: None, node_id: 0 }
    }

    
}


impl DungeonTree {
    //Return a empty tree with no nodes
    pub fn new(splits: usize) -> DungeonTree {
        DungeonTree {
            nodes: Vec::with_capacity(splits * 2),
        }
    }

    // pub fn num_children(&self, root_node: DungeonNode) -> i32
    // {
    //     let num = 0;

    //     let mut root_idx = root_node.node_id;
    //     while()
    // }

    //Sets the root of the tree
    pub fn setRoot(&mut self, root_node: DungeonNode) -> Result<(), TreeError> {
        if self.nodes.len() != 0 {
            Err(TreeError::RootErr)
        } else {
            *self = DungeonTree {
                nodes: vec![Some(root_node); 1],
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
                sub_dungeons.1.unwrap().room = None;
                continue;
            }

            let sub_width = sub_dungeons.1.unwrap().coords.unwrap().2 - sub_dungeons.1.unwrap().coords.unwrap().0;
            let sub_height = sub_dungeons.1.unwrap().coords.unwrap().3 - sub_dungeons.1.unwrap().coords.unwrap().1;

            if (sub_width <= 3 || sub_height <= 3) {
                println!("Sub dungeon is too small!");
                sub_dungeons.1.unwrap().room = None;
                continue;
            }

            //Simplistic, randomize later
            let dims = (
                sub_dungeons.1.unwrap().coords.unwrap().0 + min_x_offset,
                sub_dungeons.1.unwrap().coords.unwrap().1 + min_y_offset,
                sub_dungeons.1.unwrap().coords.unwrap().2 - max_x_offset,
                sub_dungeons.1.unwrap().coords.unwrap().3 - max_y_offset,
            );

            sub_dungeons.1.unwrap().room = Some(dims);
        }
        Ok(())
    }

    pub fn get_subtree(&self, node_idx: usize, left:bool) -> Option<DungeonTree>
    {
        if(self.nodes.len() > node_idx  && left)
        {

            let mut temp = self.clone();
            temp.nodes[node_idx] = None;
            temp.remove_at_idx((2 * node_idx + 2) as i32);

            temp.nodes.retain(|n| n.is_some());

            return Some(temp);
        }

        if(self.nodes.len() > node_idx  && !left)
        {

            let mut temp = self.clone();
            temp.nodes[node_idx] = None;
            temp.remove_at_idx((2 * node_idx + 1) as i32);

            temp.nodes.retain(|n| n.is_some());

            return Some(temp);
        }

        None
    }

    pub fn remove_at_idx(&mut self, node_idx: i32)
    {
        let mut stk: Vec<i32> = Vec::new();
        let mut toRemove: Vec<i32> = Vec::new();


        let mut curr:Option<i32> = Some(node_idx);

        while(!stk.is_empty()) || ( curr != None)
        {

            if(curr != None)
            {
                stk.push(self.nodes[curr.unwrap() as usize].unwrap().node_id as i32);

                if(self.nodes[curr.unwrap() as usize].unwrap().left == None)
                {
                    curr = None;
                }
                else {
                    curr = Some(self.nodes[curr.unwrap() as usize].unwrap().left.unwrap() as i32);
                }

                
            }

            else {
                curr = stk.last().copied();
                stk.pop();
                //print!("{}", self.nodes[curr.unwrap() as usize].unwrap().node_id);
                toRemove.push(self.nodes[curr.unwrap() as usize].unwrap().node_id as i32);
                if(self.nodes[curr.unwrap() as usize].unwrap().right == None)
                {
                    curr = None;
                }
                else {
                    curr = Some(self.nodes[curr.unwrap() as usize].unwrap().right.unwrap() as i32);
                }
            }

        }

        for idx in toRemove
        {
            self.nodes[idx as usize] = None;
        }
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
                root_node = node.unwrap().clone();
                root_idx = idx;
                node.as_mut().unwrap().left = Some(2 * idx + 1);
                node.as_mut().unwrap().right = Some(2 * idx + 2);
                if vert {
                    split_range = (node.unwrap().coords.unwrap().0, node.unwrap().coords.unwrap().2)
                } else {
                    split_range = (node.unwrap().coords.unwrap().1, node.unwrap().coords.unwrap().3)
                }
            }
            None => return Err(TreeError::IndexError),
        }

        //Check later for balance
        split_pos = (split_range.0 + split_range.1) / 2;
        split_pos = (split_pos as f32 * rand::thread_rng().gen_range(0.35..0.75)) as i32;

        self.nodes.resize(self.nodes.len() + 2, Some(root_node.clone()));
        if vert {
            self.nodes[2 * root_idx + 1] = Some(DungeonNode {
                coords: Some((
                    root_node.coords.unwrap().0 + 1,
                    root_node.coords.unwrap().1 + 1,
                    split_pos,
                    root_node.coords.unwrap().3 - 1,
                )),
                left: None,
                right: None,
                room: None,
                node_id: 2 * root_idx + 1,
            });
            self.nodes[2 * root_idx + 2] = Some(DungeonNode {
                coords: Some((
                    split_pos + 1,
                    root_node.coords.unwrap().1 + 1,
                    root_node.coords.unwrap().2 - 1,
                    root_node.coords.unwrap().3 - 1,
                )),
                left: None,
                right: None,
                room: None,
                node_id: 2 * root_idx + 2,
            });
        } else {
            self.nodes[2 * root_idx + 1] = Some(DungeonNode {
                coords: Some((
                    root_node.coords.unwrap().0 + 1,
                    root_node.coords.unwrap().1 + 1,
                    root_node.coords.unwrap().2 - 1,
                    split_pos,
                    
                )),
                left: None,
                right: None,
                room: None,
                node_id: 2 * root_idx + 1,
            });
            self.nodes[2 * root_idx + 2] = Some(DungeonNode {
                coords: Some((
                    root_node.coords.unwrap().0 + 1,
                    split_pos + 1,
                    root_node.coords.unwrap().2 - 1,
                    root_node.coords.unwrap().3 - 1,
                )),
                left: None,
                right: None,
                room: None,
                node_id: 2 * root_idx + 1,
            });
        }



        Ok(())
    }

    pub fn draw_to_file(&mut self) {
        let width = self.nodes[0].unwrap().coords.unwrap().2 - self.nodes[0].unwrap().coords.unwrap().0;
        let height = self.nodes[0].unwrap().coords.unwrap().3 - self.nodes[0].unwrap().coords.unwrap().1;

        let path = Path::new("dung.out");
        let display = path.display();
        let mut grid: Vec<Vec<char>> = vec![vec![' '; height as usize]; width as usize];
        let mut buf = String::new();

        let mut file = match File::create(&path) {
            Err(why) => panic!("couldn't create {}: {}", display, why),
            Ok(file) => file,
        };

        for sub_dungeon in self.nodes.iter().enumerate() {
            let x1 = sub_dungeon.1.unwrap().coords.unwrap().0;
            let y1 = sub_dungeon.1.unwrap().coords.unwrap().1;
            let x2 = sub_dungeon.1.unwrap().coords.unwrap().2;
            let y2 = sub_dungeon.1.unwrap().coords.unwrap().3;

            for y in y1..y2 {
                for x in x1..x2 {
                    if (y == y1 || y == y2 - 1) || (x == x1 || x == x2 - 1) {
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

        let rooms: Vec<_> = self.nodes.iter_mut().map(|c| c.unwrap().room).collect();

        for room in rooms {
            if room == None {
                continue;
            }

            let room_x1 = room.unwrap().0;
            let room_y1 = room.unwrap().1;
            let room_x2 = room.unwrap().2;
            let room_y2 = room.unwrap().3;

            for y in room_y1..room_y2 {
                for x in room_x1..room_x2 {
                    grid[y as usize][x as usize] = '.';
                }
            }
        }

        for line in grid {
            for c in line {
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

            for y in sub_dungeon.unwrap().coords.unwrap().1..=sub_dungeon.unwrap().coords.unwrap().3 {
                for x in sub_dungeon.unwrap().coords.unwrap().0..=sub_dungeon.unwrap().coords.unwrap().2 {
                    if (y == sub_dungeon.unwrap().coords.unwrap().1 || y == sub_dungeon.unwrap().coords.unwrap().3)
                        || (x == sub_dungeon.unwrap().coords.unwrap().0
                            || x == sub_dungeon.unwrap().coords.unwrap().2)
                    {
                        let _ = stdout
                            .queue(cursor::MoveTo(x.try_into().unwrap(), y.try_into().unwrap()))
                            .unwrap()
                            .queue(style::PrintStyledContent(colors[colr]));
                    }
                }
            }

            let midX = (sub_dungeon.unwrap().coords.unwrap().0 + sub_dungeon.unwrap().coords.unwrap().2) / 2;
            let midY = (sub_dungeon.unwrap().coords.unwrap().1 + sub_dungeon.unwrap().coords.unwrap().3) / 2;
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

            if (sub_dungeon.unwrap().room == None) {
                continue;
            }

            for y in sub_dungeon.unwrap().room.unwrap().1..=sub_dungeon.unwrap().room.unwrap().3 {
                for x in sub_dungeon.unwrap().room.unwrap().0..=sub_dungeon.unwrap().room.unwrap().2 {
                    let _ = stdout
                        .queue(cursor::MoveTo(x.try_into().unwrap(), y.try_into().unwrap()))
                        .unwrap()
                        .queue(style::PrintStyledContent(colors[colr]));
                }
            }

            let midX = (sub_dungeon.unwrap().room.unwrap().0 + sub_dungeon.unwrap().room.unwrap().2) / 2;
            let midY = (sub_dungeon.unwrap().room.unwrap().1 + sub_dungeon.unwrap().room.unwrap().3) / 2;
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

    fn print_tree_console(&self) 
    {
        print_tree(self);
    }
}

fn main() {
    //execute!(io::stdout(), EnterAlternateScreen);
    let mut test = DungeonTree::new(4);

    let rt: DungeonNode = DungeonNode {
        coords: Some((0, 0, 128, 128)),
        left: None,
        right: None,
        room: None,
        node_id: 0,
    };

    test.setRoot(rt);

    test.split_sub_dungeon(true, 0);
    //test.split_sub_dungeon(false, 1);
    //test.split_sub_dungeon(false, 2);
    //println!("{:?}", test.nodes);
    //test.remove_at_idx(1);
    //println!("{:?}", test.nodes);
    //test.build_rooms((4, 4, 4, 4));
    //test.draw_to_file();

    // let thing = test.get_subtree(0, false);
    // println!("{:?}", thing);

    test.print_tree_console();

    // for node in test.nodes.iter() {
    //     //println!("Sub-Dungeon coords{:?}", node.coords);
    //     println!("Room in sub-dungeon coords:{:?}", node.room);
    //     println!();
    // }

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

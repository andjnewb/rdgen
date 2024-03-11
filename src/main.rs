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
use std::{fmt::Display};
use std::path::Path;
use std::{borrow::Cow, fs::File};
use std::{
    io::{self, Write},
    task::Poll,
    time::Duration,
};
use thiserror::Error;

//use display_tree::*;

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

    #[error("No leaves found in tree...")]
    NoLeavesError,

    #[error("You must provide a Some(room)...")]
    RoomIsNoneError,
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
pub struct DungeonPath
{
    sub_paths: Vec<Option<(i32, i32)>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DungeonTree {
    nodes: Vec<Option<DungeonNode>>,
    paths: Vec<DungeonPath>,
}
#[derive(Debug)]
enum rect_face
{
    NORTH,
    SOUTH,
    EAST,
    WEST,
    NORTHEAST,
    SOUTHEAST,
    SOUTHWEST,
    NORTHWEST,
    NONE
}

impl TreeItem for DungeonTree {
    type Child = Self;
    fn write_self<W: io::Write>(&self, f: &mut W, style: &Style) -> io::Result<()> {
        write!(f, "{}", style.paint(self))
    }
    fn children(&self) -> Cow<[Self::Child]> {
        let left_subtree = self.get_subtree(0, true);
        let right_subtree = self.get_subtree(0, false);

        if (left_subtree != None) && (right_subtree != None) {
            return Cow::from(vec![left_subtree.unwrap(), right_subtree.unwrap()]);
        }

        if (left_subtree != None) && (right_subtree == None) {
            return Cow::from(vec![left_subtree.unwrap()]);
        }

        if (left_subtree == None) && (right_subtree != None) {
            return Cow::from(vec![right_subtree.unwrap()]);
        }

        Cow::from(vec![])
    }
}

impl std::fmt::Display for DungeonTree {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        // if (self.nodes.len() > 0) {
        //     write!(fmt, "Coords: {:?}", self.nodes[0].unwrap().coords)?;
        //     if (self.nodes[0].unwrap().room != None) {
        //         write!(fmt, "Room: {:?}", self.nodes[0].unwrap().room.unwrap())
        //     } else {
        //         write!(fmt, "Room: {:?}", None::<DungeonNode>)
        //     }
        // } else {
        //     write!(fmt, "None")
        // }
        write!(fmt, "Coords: {:?}", self.nodes[0])
    }
}

impl DungeonNode {
    pub fn new() -> DungeonNode {
        DungeonNode {
            coords: None,
            left: None,
            right: None,
            room: None,
            node_id: 0,
        }
    }
}

impl DungeonTree {
    //Return a empty tree with no nodes
    pub fn new(splits: usize) -> DungeonTree {
        DungeonTree {
            nodes: Vec::with_capacity(splits * 2),
            paths: Vec::new()
        }
    }

    // pub fn num_children(&self, root_node: DungeonNode) -> i32
    // {
    //     let num = 0;

    //     let mut root_idx = root_node.node_id;
    //     while()
    // }


    pub fn gen_paths(& mut self) -> Result<(), TreeError>
    {

        let parents_with_leaves: Vec<(usize, &Option<DungeonNode>)> = self.nodes.iter().enumerate()
        .filter(|(_, node)| node.is_some())
        .filter(|(_,node)| node.unwrap().left.is_some() && node.unwrap().right.is_some())
        .filter(|(idx, node)| 
        self.nodes[node.unwrap().left.unwrap()].unwrap().left.is_none()
        &&
        self.nodes[node.unwrap().left.unwrap()].unwrap().right.is_none()
        && 
        self.nodes[node.unwrap().right.unwrap()].unwrap().left.is_none()
        && 
        self.nodes[node.unwrap().right.unwrap()].unwrap().right.is_none()
        )
        .collect();

        if(parents_with_leaves.is_empty())
        {
            return Err(TreeError::NoLeavesError);
        }

        for (idx, node) in parents_with_leaves
        {
            let left = self.nodes[node.unwrap().left.unwrap()];
            let right = self.nodes[node.unwrap().right.unwrap()];

            let center_left: (i32,i32) = (left.unwrap().coords.unwrap().2 / 2, left.unwrap().coords.unwrap().3 / 2);
            let center_right: (i32,i32) = (right.unwrap().coords.unwrap().2 / 2, right.unwrap().coords.unwrap().3 / 2);
            
            let pth = DungeonTree::get_path(center_left, center_right);
            self.paths.push(pth);
        }


        Ok(())
    }

    fn get_path(point_1: (i32,i32), point_2: (i32, i32)) -> DungeonPath
    {
        let mut path: DungeonPath = DungeonPath{sub_paths: Vec::new()};

        if(point_1.1 == point_2.1)
        {
            
            match point_1.0 <= point_2.0
            {
                true => {
                    let mut x = point_1.0;
                    
                    while(x <= point_2.0)
                    {
                        path.sub_paths.push(Some((x, point_1.1)));
                        x += 1;
                    }
                },
                false => {
                    let mut x = point_1.0;
                    
                    while(x >= point_2.0)
                    {
                        path.sub_paths.push(Some((x, point_1.1)));
                        x -= 1;
                    }

                }
            }
            return path;
        }

        else if(point_1.0 == point_2.0)
        {
            
            match point_1.1 <= point_2.1
            {
                true => {
                    let mut y = point_1.1;
                    
                    while(y <= point_2.1)
                    {
                        path.sub_paths.push(Some((point_1.0, y)));
                        y += 1;
                    }
                },
                false => {
                    let mut y = point_1.1;
                    
                    while(y >= point_2.1)
                    {
                        path.sub_paths.push(Some((point_1.0, y)));
                        y -= 1;
                    }

                }
            }
            return path;
        }

        else {
            let pt_1 = (point_1.0 as f32, point_1.1 as f32);
            let pt_2 = (point_2.0 as f32, point_2.1 as f32);
            
            let distance =  f32::sqrt(f32::powf(pt_2.0 - pt_1.0, 2.0) + f32::powf(pt_2.1 - pt_1.1, 2.0));
    
            let first_leg_distance = (f32::cos(0.785398) * distance) as i32;
            let second_leg_distance = (f32::sin(0.785398) * distance) as i32;

            let mut midpoint: (i32,i32);
            let mut endpoint: (i32, i32);


            match Self::get_direction_of_point(point_1, point_2) {
                rect_face::NORTHEAST => {
                    midpoint = (point_1.0 + first_leg_distance, point_1.1);
                    

                    let first_x_leg_pts = (point_1.0 ..= midpoint.0)
                    .collect::<Vec<i32>>();
                    
                    for x in first_x_leg_pts
                    {
                        path.sub_paths.push(Some((x, point_1.1)));
                    }

                    endpoint = (midpoint.0, midpoint.1 - second_leg_distance);

                    let second_y_leg_pts = (endpoint.1 ..= midpoint.1)
                    .collect::<Vec<i32>>();

                    for y in second_y_leg_pts
                    {
                        path.sub_paths.push(Some((endpoint.0, y)));
                    }

                    

                },
                rect_face::SOUTHEAST => {
                    midpoint = (point_1.0, point_1.1 + first_leg_distance);

                    let first_y_leg_pts = (point_1.1 ..= midpoint.1)
                    .collect::<Vec<i32>>();
                    
                    for y in first_y_leg_pts
                    {
                        path.sub_paths.push(Some((point_1.0, y)));
                    }

                    endpoint = (midpoint.0 + second_leg_distance, midpoint.1);

                    let second_x_leg_pts = (midpoint.0 ..= endpoint.0)
                    .collect::<Vec<i32>>();

                    for x in second_x_leg_pts
                    {
                        path.sub_paths.push(Some((x, endpoint.1)));
                    }

                },
                rect_face::SOUTHWEST => {
                    midpoint = (point_1.0 - first_leg_distance, point_1.1);

                    let first_x_leg_pts = (midpoint.0 ..= point_1.0)
                    .collect::<Vec<i32>>();
                    
                    for x in first_x_leg_pts
                    {
                        path.sub_paths.push(Some((x, point_1.1)));
                    }

                    endpoint = (midpoint.0, midpoint.1 + second_leg_distance);

                    let second_y_leg_pts = (midpoint.1 ..= endpoint.1)
                    .collect::<Vec<i32>>();

                    for y in second_y_leg_pts
                    {
                        path.sub_paths.push(Some((endpoint.0, y)));
                    }
                },
                rect_face::NORTHWEST => {
                    midpoint = (point_1.0, point_1.1 - first_leg_distance);

                    let first_y_leg_pts = (midpoint.1 ..= point_1.1)
                    .collect::<Vec<i32>>();
                    
                    for y in first_y_leg_pts
                    {
                        path.sub_paths.push(Some((point_1.0, y)));
                    }

                    endpoint = (midpoint.0 - second_leg_distance, midpoint.1);

                    let second_x_leg_pts = (endpoint.0 ..= midpoint.0)
                    .collect::<Vec<i32>>();

                    for x in second_x_leg_pts
                    {
                        path.sub_paths.push(Some((x, endpoint.1)));
                    }
                },
                _ => {}
            }


           

            return path;
        }

        
    }

    fn get_direction_of_point(point_1: (i32,i32), point_2: (i32, i32)) -> rect_face
    {
        let x1 = point_1.0;
        let x2 = point_2.0;
        let y1 = point_1.1;
        let y2 = point_2.1;

        //RIGHT SIDE
        if( x2 > x1)
        {
            if(y2 == y1)
            {
                return rect_face::EAST;
            }
            else if (y2 < y1)
            {
                return rect_face::NORTHEAST;
            }
            else if( y2 > y1)
            {
                return rect_face::SOUTHEAST;
            }
        }
        else if( x2 < x1 ) {
            if(y2 == y1)
            {
                return rect_face::WEST;
            }
            else if (y2 > y1)
            {
                return rect_face::SOUTHWEST;
            }
            else if( y2 < y1)
            {
                return rect_face::NORTHWEST;
            }
        }

        else if(x2 == x1)
        {
            if (y2 > y1)
            {
                return rect_face::SOUTH;
            }
            else if(y2 < y1)
            {
                return rect_face::NORTH;
            }
        }

        return rect_face::NONE;
    }

    // fn is_within_range(point: (i32, i32), range:(i32,i32,i32,i32)) -> bool
    // {
    //     if((point.0 >= range.0) && (point.0 <= range.2)) || ((point.1 >= range.1) && (point.1 <= range.3))
    //     {
    //         return true;
    //     }
    //     false
    // }

    // fn get_direction_of_room(room_1: (i32,i32,i32,i32), room_2: (i32,i32,i32,i32)) -> rect_face
    // {
    //     let room_1_center: (i32,i32) = (room_1.2 / 2, room_1.3 / 2);
    //     let room_2_center:(i32,i32)=  (room_2.2 / 2, room_2.3 / 2);

    //     if(room_1_center.1 > room_2_center.1 && Self::is_within_range(room_1_center, room_2))
    //     {
    //         return rect_face::SOUTH;
    //     }

    //     return rect_face::EAST;
    // }

    // //Think of this as a line, with (start of line, end of line)
    // fn get_face(room: Option<(i32,i32,i32,i32)>, face: rect_face) -> Result<Option<(i32,i32, i32, i32)>, TreeError>
    // {
    //     if(room == None)
    //     {
    //         return Err(TreeError::RoomIsNoneError);
    //     }

    //     match face{
    //         rect_face::NORTH => return Ok(Some((room.unwrap().0, room.unwrap().1, room.unwrap().2, room.unwrap().1))),
    //         rect_face::SOUTH => return Ok(Some((room.unwrap().0, room.unwrap().3, room.unwrap().2, room.unwrap().3))),
    //         rect_face::EAST =>  return Ok(Some((room.unwrap().2, room.unwrap().1, room.unwrap().2, room.unwrap().3))),
    //         rect_face::WEST =>  return Ok(Some((room.unwrap().0, room.unwrap().1, room.unwrap().0, room.unwrap().3))),
    //         _ => return Ok(None)
    //     }

    // } 

    // fn get_common_faces(room_1: Option<(i32,i32,i32,i32)>, room_2: Option<(i32,i32,i32,i32)>) -> 

    // //Stupid name, but gets the range, that two rooms share on the x or y axis respectively
    // fn get_face_range(room_1: Option<(i32,i32,i32,i32)>, room_2: Option<(i32,i32,i32,i32)>) -> Result<Option<(i32,i32,i32,i32)>, TreeError>
    // {

    //     let x_range: Option<(i32, i32)> = Some((0,0));
    //     let y_range: Option<(i32, i32)> = Some((0,0));

    //     if(room_1.is_none() || room_2.is_none())
    //     {
    //         return Err(TreeError::RoomIsNoneError);
    //     }

    //     let room_1_x_range: Option<(i32, i32)> = Some((room_1.unwrap().0,room_1.unwrap().2));
    //     let room_1_y_range: Option<(i32, i32)> = Some((room_1.unwrap().1,room_1.unwrap().3));
    //     let room_2_x_range: Option<(i32, i32)> = Some((room_2.unwrap().0,room_2.unwrap().2));
    //     let room_2_y_range: Option<(i32, i32)> = Some((room_2.unwrap().1,room_2.unwrap().3));

    //     let shares_x_points = 


    //     else {
    //         let leftmost_x: i32;
    //         if(room_1.unwrap().0 < room_2.unwrap().0)
    //         {

    //         }
    //     }

        
    //     Ok()
    // }

    //Sets the root of the tree
    pub fn setRoot(&mut self, root_node: DungeonNode) -> Result<(), TreeError> {
        if self.nodes.len() != 0 {
            Err(TreeError::RootErr)
        } else {
            *self = DungeonTree {
                nodes: vec![Some(root_node); 1],
                paths: Vec::new()
            };
            Ok(())
        }
    }

    pub fn get_leaves(&self) -> Result<Vec<Option<DungeonNode>>, TreeError>
    {
        let leaves: Vec<Option<DungeonNode>> = self.nodes.iter()
        .filter(|node| node.is_some())
        .filter(|node| (node.unwrap().left == None) && (node.unwrap().right == None))
        .map(|node| *node)
        .collect();

        if(leaves.len() <= 0)
        {
            Err(TreeError::NoLeavesError)
        }
        else {
            Ok(leaves)
        }
    }

    pub fn build_rooms(&mut self, offsets: (i32, i32, i32, i32)) -> Result<(), TreeError> {
        let min_x_offset = offsets.0;
        let min_y_offset = offsets.1;
        let max_x_offset = offsets.2;
        let max_y_offset = offsets.3;

        //Only build rooms for leaves?
        let mut itr: Vec<_> = self.nodes
        .iter_mut()
        .filter(|node| node.is_some())
        .filter(|node| (node.unwrap().left.is_none()) && (node.unwrap().right.is_none())).collect();

        for sub_dungeons in itr.iter_mut().enumerate() {
            // if (sub_dungeons.0 == 0) {
            //     sub_dungeons.1.unwrap().room = None;
            //     continue;
            // }

            if (**sub_dungeons.1 != None) {
                let sub_width = sub_dungeons.1.unwrap().coords.unwrap().2
                    - sub_dungeons.1.unwrap().coords.unwrap().0;
                let sub_height = sub_dungeons.1.unwrap().coords.unwrap().3
                    - sub_dungeons.1.unwrap().coords.unwrap().1;

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

                sub_dungeons.1.as_mut().unwrap().room = Some(dims);
            }
        }
        Ok(())
    }

    pub fn get_subtree(&self, node_idx: usize, left: bool) -> Option<DungeonTree> {
        if node_idx >= self.nodes.len() {
            return None;
        }

        let mut kids: Vec<usize> = Vec::new();

        match self.nodes[node_idx] {
            Some(node) => match left {
                true => match node.left {
                    Some(left_child_idx) => {
                        //kids.push(node_idx);
                        kids.push(left_child_idx);
                        match self.get_children_idxs(self.nodes[left_child_idx], &mut kids) {
                            Ok(_) => {
                                let mut subtree: DungeonTree = DungeonTree::new(1);

                                let mut new_node_id = 0;
                                let mut new_child_id = 1;
                                for idx in kids {
                                    let mut node: DungeonNode = self.nodes[idx].unwrap().clone();
                                    node.node_id = new_node_id;

                                    if (node.left != None) {
                                        node.left = Some(new_child_id);
                                        new_child_id += 1;
                                    }

                                    if (node.right != None) {
                                        node.right = Some(new_child_id);
                                        new_child_id += 1;
                                    }

                                    subtree.nodes.push(Some(node));

                                    new_node_id += 1;
                                }

                                return Some(subtree);
                            }
                            _ => {}
                        }
                    }
                    None => {
                        return None;
                    }
                },
                false => {
                    match node.right {
                        Some(right_child_idx) => {
                            //kids.push(node_idx);
                            kids.push(right_child_idx);
                            match self.get_children_idxs(self.nodes[right_child_idx], &mut kids) {
                                Ok(_) => {
                                    let mut subtree: DungeonTree = DungeonTree::new(1);

                                    let mut new_node_id = 0;
                                    let mut new_child_id = 1;
                                    for idx in kids {
                                        let mut node: DungeonNode =
                                            self.nodes[idx].unwrap().clone();
                                        node.node_id = new_node_id;

                                        if (node.left != None) {
                                            node.left = Some(new_child_id);
                                            new_child_id += 1;
                                        }

                                        if (node.right != None) {
                                            node.right = Some(new_child_id);
                                            new_child_id += 1;
                                        }

                                        subtree.nodes.push(Some(node));

                                        new_node_id += 1;
                                    }

                                    return Some(subtree);
                                }
                                _ => {}
                            }
                        }
                        None => {
                            return None;
                        }
                    }
                }
            },

            None => {
                return None;
            }
        }

        None
    }

    pub fn get_children_idxs(
        &self,
        rt: Option<DungeonNode>,
        child_idxs: &mut Vec<usize>,
    ) -> Result<(), TreeError> {
        if (rt == None) {
            return Ok(());
        }

        if (rt.unwrap().left != None) {
            child_idxs.push(rt.unwrap().left.unwrap());
            self.get_children_idxs(self.nodes[rt.unwrap().left.unwrap()], child_idxs)?;
        }

        if (rt.unwrap().right != None) {
            child_idxs.push(rt.unwrap().right.unwrap());
            self.get_children_idxs(self.nodes[rt.unwrap().right.unwrap()], child_idxs)?;
        }

        Ok(())
    }

    pub fn remove_at_idx(&mut self, node_idx: i32) {
        let mut stk: Vec<usize> = Vec::new();
        let mut toRemove: Vec<i32> = Vec::new();

        let mut curr: Option<usize> = Some(node_idx as usize);

        while (!stk.is_empty()) || (curr != None) {
            match curr {
                Some(_) => {
                    //println!("{:?}", curr.unwrap());
                    match self.nodes[curr.unwrap() as usize] {
                        Some(node) => {
                            stk.push(node.node_id);

                            match node.left {
                                Some(left_child_idx) => curr = Some(left_child_idx),
                                None => curr = None,
                            };
                        }
                        None => {}
                    };
                }
                None => {
                    curr = stk.last().copied();
                    stk.pop();

                    match self.nodes[curr.unwrap() as usize] {
                        Some(node) => {
                            toRemove.push(node.node_id as i32);

                            match node.right {
                                Some(right_child_idx) => curr = Some(right_child_idx),
                                None => {
                                    curr = None;
                                }
                            }
                        }
                        None => {}
                    }
                }
            };

            // if(curr != None)
            // {
            //     if(curr.unwrap() < self.nodes.len() as i32 && self.nodes[curr.unwrap() as usize] != None)
            //     {
            //         stk.push(self.nodes[curr.unwrap() as usize].unwrap().node_id as i32 & self.nodes.len() as i32);
            //     }

            //     if(self.nodes[curr.unwrap() as usize].unwrap().left == None)
            //     {
            //         curr = None;
            //     }
            //     else {
            //         curr = Some(self.nodes[curr.unwrap() as usize].unwrap().left.unwrap() as i32);
            //     }

            // }

            // else {
            //     curr = stk.last().copied();
            //     stk.pop();
            //     //print!("{}", self.nodes[curr.unwrap() as usize].unwrap().node_id);
            //     toRemove.push(self.nodes[curr.unwrap() as usize].unwrap().node_id as i32);
            //     if(self.nodes[curr.unwrap() as usize].unwrap().right == None)
            //     {
            //         curr = None;
            //     }
            //     else {
            //         curr = Some(self.nodes[curr.unwrap() as usize].unwrap().right.unwrap() as i32);
            //     }
            // }
        }

        for idx in toRemove {
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
                    split_range = (
                        node.unwrap().coords.unwrap().0,
                        node.unwrap().coords.unwrap().2,
                    )
                } else {
                    split_range = (
                        node.unwrap().coords.unwrap().1,
                        node.unwrap().coords.unwrap().3,
                    )
                }
            }
            None => return Err(TreeError::IndexError),
        }

        //Check later for balance
        split_pos = (split_range.0 + split_range.1) / 2;
        split_pos = (split_pos as f32 * rand::thread_rng().gen_range(0.35..0.75)) as i32;

        self.nodes.resize(self.nodes.len() + 3, None);
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
                node_id: 2 * root_idx + 2,
            });
        }

        Ok(())
    }

    pub fn draw_to_file(&mut self) {
        let width =
            self.nodes[0].unwrap().coords.unwrap().2 - self.nodes[0].unwrap().coords.unwrap().0;
        let height =
            self.nodes[0].unwrap().coords.unwrap().3 - self.nodes[0].unwrap().coords.unwrap().1;

        let path = Path::new("dung.out");
        let display = path.display();
        let mut grid: Vec<Vec<char>> = vec![vec![' '; height as usize]; width as usize];
        let mut buf = String::new();
        let mut rooms: Vec<Option<(i32, i32, i32, i32)>> = Vec::new();

        let mut file = match File::create(&path) {
            Err(why) => panic!("couldn't create {}: {}", display, why),
            Ok(file) => file,
        };

        let itr = self.nodes.iter()
        .filter(|node| node.is_some())
        .filter(|node| node.unwrap().room.is_some());

        for sub_dungeon in itr.enumerate() {
            let x1 = sub_dungeon.1.unwrap().room.unwrap().0;
            let y1 = sub_dungeon.1.unwrap().room.unwrap().1;
            let x2 = sub_dungeon.1.unwrap().room.unwrap().2;
            let y2 = sub_dungeon.1.unwrap().room.unwrap().3;

            for y in y1..y2 {
                for x in x1..x2 {
                    if (y == y1 || y == y2 - 1) || (x == x1 || x == x2 - 1) {
                        grid[y as usize][x as usize] = '*';
                    } else {
                        grid[y as usize][x as usize] = ' ';
                    }
                }
            }

            rooms.push(sub_dungeon.1.unwrap().room);
            // let midX = (sub_dungeon.coords.unwrap().0 + sub_dungeon.coords.unwrap().2) / 2;
            // let midY = (sub_dungeon.coords.unwrap().1 + sub_dungeon.coords.unwrap().3) / 2;
            //Print node name
            // let _ = stdout
            //                 .queue(cursor::MoveTo(midX.try_into().unwrap(),midY.try_into().unwrap())).unwrap()
            //                 .queue(style::Print(sub_dung_lbl));
        }

        
       

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

            for y in sub_dungeon.unwrap().coords.unwrap().1..=sub_dungeon.unwrap().coords.unwrap().3
            {
                for x in
                    sub_dungeon.unwrap().coords.unwrap().0..=sub_dungeon.unwrap().coords.unwrap().2
                {
                    if (y == sub_dungeon.unwrap().coords.unwrap().1
                        || y == sub_dungeon.unwrap().coords.unwrap().3)
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

            let midX = (sub_dungeon.unwrap().coords.unwrap().0
                + sub_dungeon.unwrap().coords.unwrap().2)
                / 2;
            let midY = (sub_dungeon.unwrap().coords.unwrap().1
                + sub_dungeon.unwrap().coords.unwrap().3)
                / 2;
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
        
        cpy.retain(|c| *c != None);

        stdout.execute(terminal::Clear(terminal::ClearType::All)).unwrap();

        let mut sub_dung_lbl = 0;
        for sub_dungeon in cpy {
            let colr = sub_dung_lbl % colors.len();

            if (sub_dungeon.unwrap().room == None) {
                continue;
            }

            for y in sub_dungeon.unwrap().room.unwrap().1..=sub_dungeon.unwrap().room.unwrap().3 {
                for x in sub_dungeon.unwrap().room.unwrap().0..=sub_dungeon.unwrap().room.unwrap().2
                {
                    let _ = stdout
                        .queue(cursor::MoveTo(x.try_into().unwrap(), y.try_into().unwrap()))
                        .unwrap()
                        .queue(style::PrintStyledContent(colors[colr]));
                }
            }

            let midX =
                (sub_dungeon.unwrap().room.unwrap().0 + sub_dungeon.unwrap().room.unwrap().2) / 2;
            let midY =
                (sub_dungeon.unwrap().room.unwrap().1 + sub_dungeon.unwrap().room.unwrap().3) / 2;
            //Print node name
            let _ = stdout
                .queue(cursor::MoveTo(
                    midX.try_into().unwrap(),
                    midY.try_into().unwrap(),
                ))
                .unwrap()
                .queue(style::Print(sub_dungeon.unwrap().node_id));
            sub_dung_lbl += 1;
        }
        stdout.flush().unwrap();
    }

    fn print_tree_console(&self) {
        print_tree(self);
    }

    pub fn draw_paths(&self)
    {
        let mut stdout = io::stdout();

        for path in self.paths.clone()
        {
            for point in path.sub_paths
            {
                let _ = stdout
                        .queue(cursor::MoveTo(point.unwrap().0 as u16, point.unwrap().1 as u16))
                        .unwrap()
                        .queue(style::PrintStyledContent("█".white()));
            }
        }

        stdout.flush().unwrap();
    }
}

fn main() {
    execute!(io::stdout(), EnterAlternateScreen).unwrap();
    let mut test = DungeonTree::new(4);

    let rt: DungeonNode = DungeonNode {
        coords: Some((0, 0, 64, 64)),
        left: None,
        right: None,
        room: None,
        node_id: 0,
    };

    test.setRoot(rt);

    test.split_sub_dungeon(true, 0);
    test.split_sub_dungeon(false, 1);
     test.split_sub_dungeon(false, 2);
    // test.split_sub_dungeon(true, 3);
    // test.split_sub_dungeon(false, 4);
    test.build_rooms((2, 2, 2, 2));
    test.draw_rooms();
    test.gen_paths();
    test.draw_paths();

    //test.print_tree_console();

       loop {
        if poll(Duration::from_millis(100)).unwrap() {
            // It's guaranteed that `read` won't block, because `poll` returned
            // `Ok(true)`.
             let ev: crossterm::event::Event = crossterm::event::read().unwrap();
             let mut tt: crossterm::event::KeyCode = crossterm::event::KeyCode::Enter;

             match ev
             {
                Event::Key(key) => tt = key.code,
                _ => (),
             }

             if(tt == crossterm::event::KeyCode::Char('c'))
             {
                break;
             }

        } else {
            // Timeout expired, no `Event` is available
        }
    }
    // DungeonTree::get_path((30,30), (40, 20));

    execute!(io::stdout(), LeaveAlternateScreen).unwrap();
    test.print_tree_console();
    
}

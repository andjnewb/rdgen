use rand::Rng;
use thiserror::Error;
use std::io::{self, Write};
use crossterm::{
    ExecutableCommand, QueueableCommand,
    terminal, cursor, style::{self, Stylize}
};
struct Dungeon
{
    tree: DungeonTree,
    homogeneity: f64, //Will be clamped between 0 and 1. Increases the variance for the splitting of sub-dungeons.
    splits: i64, //How many times the sub-dungeons will be split. Going to high with too small of a dungeon can produce odd results.
    split_direction: split_dirs, // Split the sub-dungeons horziontally or vertically.
    
}

enum split_dirs
{
    ALWAYS_VERT,
    ALWAYS_HORIZONTAL,
    RANDOM,
}

#[derive(Debug, Error)]
pub enum TreeError
{
    #[error("Tree already has root node... ")]
    RootErr,

    #[error("Invalid index for tree...")]
    IndexError,

}



#[derive(Clone, Debug)]
pub struct DungeonNode
{
    //x1, y1, x2, y2
    coords: Option<(i32, i32, i32, i32)>,
    left: Option<usize>,
    right: Option<usize>,
}

pub struct DungeonTree
{
    nodes: Vec<DungeonNode>,
}

impl DungeonTree
{
    //Return a empty tree with no nodes
    pub fn new (splits: usize) -> DungeonTree
    {
        DungeonTree{nodes: Vec::with_capacity(splits * 2)}
    }

    //Sets the root of the tree
    pub fn setRoot(& mut self, root_node: DungeonNode,) -> Result<(), TreeError>
    {
        if self.nodes.len() != 0
        {
            Err(TreeError::RootErr)
        }
        else {
            *self = DungeonTree{nodes: vec![root_node;1]};
            Ok(())
        }
    }

    //At the given node, split it into two sub-dungeons. If sub-dungeons already exist at the child node locations, they will be over-written.
    //Be careful with this, as your DungeonTree node vector will continue to increase in size even if it isn't necessary.
    pub fn split(&mut self, vert: bool, node_idx: i32) -> Result<(), TreeError>
    {
        let mut split_pos: i32;
        let mut split_range: (i32, i32);
        let root_idx: usize;
        let root_node: DungeonNode;

        match self.nodes.iter_mut().enumerate().find(|c| c.0 == node_idx as usize)
        {
            Some((idx, node)) => {
                root_node = node.clone();
                root_idx = idx;
                node.left = Some(2 * idx + 1);
                node.right = Some(2 *idx + 2);
                if vert {
                split_range = (node.coords.unwrap().0, node.coords.unwrap().2)} 
                else {
                split_range = (node.coords.unwrap().1, node.coords.unwrap().3)}},
            None => return Err(TreeError::IndexError),
        }

        //Check later for balance
        split_pos = (split_range.0 + split_range.1) / 2;
        split_pos = (split_pos as f32 * rand::thread_rng().gen_range(0.25 .. 0.9)) as i32;

        
        self.nodes.resize(self.nodes.len() + 2, root_node.clone());
        if(vert)
        {
            
            self.nodes[2 * root_idx + 1] = DungeonNode{
                coords: 
                Some((root_node.coords.unwrap().0, root_node.coords.unwrap().1 , split_pos, root_node.coords.unwrap().3)),
                left: None,
                right: None, };
            self.nodes[2 * root_idx + 2] = DungeonNode{
                coords: 
                Some((split_pos, root_node.coords.unwrap().1, root_node.coords.unwrap().2, root_node.coords.unwrap().3)),
                left: None,
                right: None, }
            
        }
        else {
            self.nodes[2 * root_idx + 1] = DungeonNode{
                coords: 
                Some((root_node.coords.unwrap().0, root_node.coords.unwrap().1 , root_node.coords.unwrap().2, split_pos)),
                left: None,
                right: None, };
            self.nodes[2 * root_idx + 2] = DungeonNode{
                coords: 
                Some((root_node.coords.unwrap().0, split_pos, root_node.coords.unwrap().2, root_node.coords.unwrap().3)),
                left: None,
                right: None, }
        }
        println!();
        Ok(())
    }

    pub fn draw_sub_dungeons(&self) 
    {

        let mut stdout = io::stdout();

        let colors = ["█".magenta(), "█".red(), "█".blue(), "█".white(), "█".green(), "█".yellow()];
        
        //Skip drawing the base
        let mut cpy = self.nodes.clone();
        //cpy.remove(0);

        stdout.execute(terminal::Clear(terminal::ClearType::All)).unwrap();

        let mut sub_dung_lbl = 0;
        for sub_dungeon in cpy
        {
            let colr = sub_dung_lbl % colors.len();

            for y in sub_dungeon.coords.unwrap().1 .. sub_dungeon.coords.unwrap().3
            {
                for x in sub_dungeon.coords.unwrap().0 .. sub_dungeon.coords.unwrap().2
                {
                    
                    let _ = stdout
                            .queue(cursor::MoveTo(x.try_into().unwrap(),y.try_into().unwrap())).unwrap()
                            .queue(style::PrintStyledContent( colors[colr]));
                }
            }

            let midX = (sub_dungeon.coords.unwrap().0 + sub_dungeon.coords.unwrap().2) / 2;
            let midY = (sub_dungeon.coords.unwrap().1 + sub_dungeon.coords.unwrap().3) / 2;
            //Print node name
            let _ = stdout
                            .queue(cursor::MoveTo(midX.try_into().unwrap(),midY.try_into().unwrap())).unwrap()
                            .queue(style::Print(sub_dung_lbl));
            sub_dung_lbl += 1;

        }
        stdout.flush().unwrap();
    }
    
}




fn main()
{
   let mut test = DungeonTree::new(4);

   let rt: DungeonNode = DungeonNode {coords: Some((0, 0 , 128, 128)), left: None, right: None};

   test.setRoot(rt);

   test.split(true, 0);
   test.split(false, 1);
//    test.split(false, 2);
//    test.split(false, 3);

   test.draw_sub_dungeons();
   

}



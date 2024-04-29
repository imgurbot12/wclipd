//! Ascii Table Generation

use std::{collections::HashMap, str::FromStr};

// indexes to table components
static TABLE_JOIN: usize = 0;
static TABLE_EDGE: usize = 1;

static TABLE_TOP_LEFT: usize = 2;
static TABLE_TOP_RIGHT: usize = 3;
static TABLE_TOP_JOIN: usize = 4;

static TABLE_BTM_LEFT: usize = 5;
static TABLE_BTM_RIGHT: usize = 6;
static TABLE_BTM_JOIN: usize = 7;

// supported table styles
type StyleArray = [&'static str; 8];
static STANDARD_TABLE: StyleArray = ["|", "-", "+", "+", "+", "+", "+", "+"];
static FANCY_TABLE: StyleArray = ["│", "─", "┌", "┐", "┬", "└", "┘", "┴"];

#[derive(Debug, Clone)]
pub enum Style {
    Standard,
    Fancy,
}

impl Default for Style {
    fn default() -> Self {
        Self::Fancy
    }
}

impl FromStr for Style {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "standard" => Ok(Self::Standard),
            "fancy" => Ok(Self::Fancy),
            _ => Err(format!("invalid style: {s:?}")),
        }
    }
}

impl Style {
    fn array(&self) -> StyleArray {
        match self {
            Self::Standard => STANDARD_TABLE,
            Self::Fancy => FANCY_TABLE,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Align {
    Left,
    Right,
    Center,
}

impl Default for Align {
    fn default() -> Self {
        Align::Left
    }
}

impl FromStr for Align {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "center" => Ok(Self::Center),
            _ => Err(format!("invalid style: {s:?}")),
        }
    }
}

pub type Entry<'a> = String;
pub type Row<'a> = Vec<Entry<'a>>;
pub type Table<'a> = Vec<Row<'a>>;

#[inline]
fn repeat(c: &str, num: usize) -> String {
    (0..num).map(|_| c).collect()
}

fn align(entry: Entry, size: usize, fill: &str, align: &Align) -> String {
    let buf = size - entry.chars().count();
    match align {
        Align::Left => format!("{fill}{entry}{fill}{}", repeat(fill, buf)),
        Align::Right => format!("{}{fill}{entry}{fill}", repeat(fill, buf)),
        Align::Center => {
            let half = buf as f64 / 2.0;
            let left = repeat(fill, half.ceil() as usize);
            let right = repeat(fill, half.floor() as usize);
            format!("{left}{fill}{entry}{fill}{right}")
        }
    }
}

/// Ascii Table Generator Utility
pub struct AsciiTable {
    title: String,
    style: StyleArray,
    align: HashMap<usize, Align>,
}

impl AsciiTable {
    /// Spawn New Ascii Table Generator
    pub fn new(title: String, style: Style) -> Self {
        Self {
            title,
            style: style.array(),
            align: HashMap::new(),
        }
    }

    /// Configure Column Default Alignment
    pub fn align_column(&mut self, col: usize, align: Align) {
        self.align.insert(col, align);
    }

    /// Draw a Single Table Row
    fn draw_row(
        &self,
        row: Row,
        fill: &str,
        start: &str,
        join: &str,
        end: &str,
        col_sizes: &Vec<usize>,
        algn: Option<&Align>,
    ) -> String {
        let mut cols = vec![];
        for (i, col) in row.into_iter().enumerate() {
            let size = col_sizes[i];
            let algn = algn.or(self.align.get(&i)).unwrap_or(&Align::Left);
            let render = align(col, size, fill, algn);
            cols.push(render);
        }
        format!("{start}{}{end}", cols.join(join))
    }

    /// Draw Ascii Table with Specified Table Values
    pub fn draw(&self, table: Table) -> String {
        // calculate size of columns
        let num_columns = table
            .iter()
            .map(|r| r.len())
            .max()
            .expect("empty table rows");
        let col_sizes: Vec<usize> = (0..num_columns)
            .map(|index| {
                table
                    .iter()
                    .map(|x| x.get(index).map(|s| s.chars().count()).unwrap_or(0))
                    .max()
                    .expect("empty table columns")
            })
            .collect();
        // draw top-row of table
        let mut lines = vec![];
        let edge_row: Row = col_sizes.iter().map(|_| Entry::default()).collect();
        let mut start_row = edge_row.clone();
        start_row[col_sizes.len() / 2] = format!(" {} ", self.title);
        lines.push(self.draw_row(
            start_row,
            self.style[TABLE_EDGE],
            self.style[TABLE_TOP_LEFT],
            self.style[TABLE_TOP_JOIN],
            self.style[TABLE_TOP_RIGHT],
            &col_sizes,
            Some(&Align::Center),
        ));
        // draw table row for row using column sizes
        lines.extend(table.into_iter().map(|row| {
            self.draw_row(
                row,
                " ",
                self.style[TABLE_JOIN],
                self.style[TABLE_JOIN],
                self.style[TABLE_JOIN],
                &col_sizes,
                None,
            )
        }));
        // draw bottom of table
        lines.push(self.draw_row(
            edge_row,
            self.style[TABLE_EDGE],
            self.style[TABLE_BTM_LEFT],
            self.style[TABLE_BTM_JOIN],
            self.style[TABLE_BTM_RIGHT],
            &col_sizes,
            None,
        ));
        lines.join("\n")
    }

    /// Draw and Print Table to Stdout
    pub fn print(&self, table: Table) {
        println!("{}", self.draw(table))
    }
}

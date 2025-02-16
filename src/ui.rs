use std::collections::HashMap;

use crate::{Col, NiceKeyboard, Window};

/// create an ui context, represented by an empty UIBox, which has a BoxPlacementStyle of Full
pub fn ui_context(start: (i32, i32), end: (i32, i32), data: UIData) -> UIRoot {
    //Box
    UIRoot {
        //UIBox
        start,
        end,
        ui_box: UIBox::default(), /*fill_style:FillStyle::default(),
                                  placement_style:BoxPlacementStyle::Full*/
        data,
    }
}
/// Root node of the ui tree
pub struct UIRoot {
    pub start: (i32, i32),
    pub end: (i32, i32),
    pub ui_box: UIBox,
    pub data: UIData,
}
impl UIRoot {
    /// Render the tree
    pub fn render_and_process(&mut self, window: &mut Window, keyboard: &NiceKeyboard) {
        self.ui_box
            .render_and_process(self.start, self.end, window, keyboard, &mut self.data);
        self.data.last_mouse_position = keyboard.mouse_position; //l
    }
    pub fn retrieve_data(self) -> UIData {
        self.data
    }
}
/// UI Box component, akin to an HTML div.
/// The nodes it adds to itself will be placed following the BoxPlacementStyle
/// The default `UIBox` is transparent, 0-sized, and follows BoxPlacementStyle::Full
pub struct UIBox {
    /*pub start:(i32,i32),//SplitY//i
    pub end:(i32,i32),*/
    pub fill_style: FillStyle,
    pub placement_style: BoxPlacementStyle,
    pub childs: Vec<Box<dyn UI>>,
}
impl Default for UIBox {
    fn default() -> Self {
        UIBox {
            /*start:(0,0),
            end:(0,0),*/
            fill_style: FillStyle::default(),
            placement_style: BoxPlacementStyle::Full, //SplitY,//i
            childs: Vec::new(),
        }
    }
}
impl UI for UIBox {
    fn render_and_process(
        &mut self,
        start: (i32, i32),
        end: (i32, i32),
        window: &mut Window,
        keyboard: &NiceKeyboard,
        data: &mut UIData,
    ) {
        self.fill_style.fill(start, end, window);
        match self.placement_style {
            BoxPlacementStyle::Full => {
                for i in &mut self.childs {
                    i.render_and_process(start, end, window, keyboard, data);
                }
            }
            BoxPlacementStyle::Within { padding } => {
                for i in &mut self.childs {
                    i.render_and_process(
                        (start.0 + padding, start.1 + padding),
                        (end.0 - padding, end.1 - padding),
                        window,
                        keyboard,
                        data,
                    );
                }
            }
            BoxPlacementStyle::AlignY { height } => {
                let mut y = start.1;
                for i in &mut self.childs {
                    i.render_and_process((start.0, y), (end.0, y + height), window, keyboard, data);
                    y += height;
                }
            }
            BoxPlacementStyle::AlignX { width } => {
                let mut x = start.0;
                for i in &mut self.childs {
                    //0
                    i.render_and_process((x, start.1), (x + width, end.1), window, keyboard, data);
                    x += width;
                }
            }
            BoxPlacementStyle::SplitY => todo!(),
            BoxPlacementStyle::SplitX => todo!(),
        }
        /*let mut x=0;
        let mut y=0;
        for i in &mut self.childs{
            i.render(start, end, window);
        }*/
    }
}
/// ui component that can have multiple childs
pub trait UINode {
    fn add<T, K>(&mut self, node: T, fun: K)
    where
        T: UI + 'static,
        K: FnMut(T) -> T;
}
impl UINode for UIRoot {
    fn add<T, K>(&mut self, node: T, fun: K)
    where
        T: UI + 'static,
        K: FnMut(T) -> T,
    {
        self.ui_box.add(node, fun);
    }
}
impl UINode for UIBox {
    fn add<T, K>(&mut self, node: T, mut fun: K)
    where
        T: UI + 'static, //Sized+
        K: FnMut(T) -> T,
    {
        self.childs.push(Box::<T>::new(fun(node)));
    }
}
/// renderable ui component

pub trait UI {
    fn render_and_process(
        &mut self,
        start: (i32, i32),
        end: (i32, i32),
        window: &mut Window,
        keyboard: &NiceKeyboard,
        data: &mut UIData,
    ); //mut
}
/// default fillstyle fills nothing
pub struct FillStyle {
    pub background_color: Option<Col>,
    pub foreground_color: Option<Col>,
    pub fill_char: Option<char>,
    pub border: BorderStyle,
}
impl FillStyle {
    /// Fill the style
    pub fn fill(&self, start: (i32, i32), end: (i32, i32), window: &mut Window) {
        for i in start.0..end.0 {
            for j in start.1..end.1 {
                if let Some(k) = self.fill_char {
                    window.set_char_at(i, j, k);
                }
                if let Some(k) = self.background_color {
                    window.set_bg_at(i, j, k);
                }
                if let Some(k) = self.foreground_color {
                    window.set_fg_at(i, j, k);
                }
            }
        }
        for i in start.0..end.0 {
            {
                let x = i;
                let y = start.1;
                if let Some(k) = self.border.bg {
                    window.set_bg_at(x, y, k);
                }
                if let Some(k) = self.border.fg {
                    window.set_fg_at(x, y, k); //b
                }
                if let Some(k) = self.border.char {
                    window.set_char_at(x, y, k); //b
                }
            }
            {
                let x = i;
                let y = end.1 - 1;
                if let Some(k) = self.border.bg {
                    window.set_bg_at(x, y, k);
                }
                if let Some(k) = self.border.fg {
                    window.set_fg_at(x, y, k); //b
                }
                if let Some(k) = self.border.char {
                    window.set_char_at(x, y, k); //b
                }
            }
        }
        for i in start.1..end.1 {
            {
                let x = start.0; //1
                let y = i;
                if let Some(k) = self.border.bg {
                    window.set_bg_at(x, y, k);
                }
                if let Some(k) = self.border.fg {
                    window.set_fg_at(x, y, k); //b
                }
                if let Some(k) = self.border.char {
                    window.set_char_at(x, y, k); //b
                }
            }
            {
                let x = end.0 - 1; //1
                let y = i;
                if let Some(k) = self.border.bg {
                    window.set_bg_at(x, y, k);
                }
                if let Some(k) = self.border.fg {
                    window.set_fg_at(x, y, k); //b
                }
                if let Some(k) = self.border.char {
                    window.set_char_at(x, y, k); //b
                }
            }
        }
    }
}
impl Default for FillStyle {
    fn default() -> Self {
        Self {
            background_color: None,
            foreground_color: None,
            fill_char: None,
            border: BorderStyle::empty(),
        }
    }
}
/// A border style
pub struct BorderStyle {
    pub char: Option<char>,
    pub fg: Option<Col>,
    pub bg: Option<Col>,
}
impl BorderStyle {
    /// Empty border that does nothing, useful for blank boxes
    pub fn empty() -> Self {
        Self {
            char: None,
            fg: None,
            bg: None,
        }
    }
}
/// Describes how the childs you add to a node will be placed
pub enum BoxPlacementStyle {
    /// Childs will occupy the same size as self
    Full,
    /// Childs will occupy the same size as self - `padding` char in every direction
    Within { padding: i32 },
    /// Childs will be placed one after the other, each occupying `height` space on the Y axis
    AlignY { height: i32 },
    /// Same thing but for the x axis
    AlignX { width: i32 },
    /// Each node is given an equal fraction of the height of the parent
    SplitY,
    /// Each node is given an equal fraction of the width of the parent
    SplitX,
}
/// One-line text label
/// The default label is transparent (actually writes nothing except glyphs) and has String::new() as text
pub struct Label {
    pub foreground_color: Option<Col>,
    pub background_color: Option<Col>,
    pub text: String,
}
impl Default for Label {
    fn default() -> Self {
        Self {
            foreground_color: None,
            background_color: None,
            text: String::new(), //(&self, other)
        }
        //todo!()
    }
}
impl UI for Label {
    fn render_and_process(
        &mut self,
        start: (i32, i32),
        _end: (i32, i32),
        window: &mut Window,
        _keyboard: &NiceKeyboard,
        _data: &mut UIData,
    ) {
        window.print_at(
            start.0,
            start.1,
            &self.text,
            self.foreground_color,
            self.background_color,
        ); //todo!()//window.
    }
}
/// Represents an identifier for an UI element, which allows for data keeping
pub type ID = String; //pub struct ID(pub String);
/// Allows for the continuation of state elements such as text inputs, pressed status for buttons and such
/// It is also how you read the data
/// Finally, it also contains the config of the UI
pub struct UIData {
    /// Root is a reserved name!
    pub selected: Option<ID>,
    pub data: HashMap<ID, UIDataEntry>,
    /// This list is never cleared (you have to do it yourself)
    pub events: Vec<Event>,
    pub config: UIConfig,
    /// The last mouse position. When the mouse moves, we test for selection
    /// You shouldn't change this yourself
    pub last_mouse_position: (i32, i32),
}
impl UIData {
    /// Appends an event
    pub fn event(&mut self, event: Event) {
        self.events.push(event);
    }
}
impl Default for UIData {
    fn default() -> Self {
        Self {
            selected: Some("root".into()), //None
            data: HashMap::new(),
            events: Vec::new(),
            config: UIConfig::default(),
            last_mouse_position: (0, 0),
        } //todo!()
    }
}
/// Represents data from some magic UI element
pub enum UIDataEntry {
    /// Example: text from a text input
    Text(String),
    /// Example: state of a button
    Boolean(bool),
}
/// A button
pub struct Button {
    pub foreground_color: Option<Col>,
    pub background_color: Option<Col>,
    pub text: String,
    pub id: ID, //String
    pub pressed_style: PressedStyle,
    pub keybind: Option<char>,
    pub decoration_left: Option<SingleCharDecoration>,
    pub decoration_right: Option<SingleCharDecoration>,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            foreground_color: None,
            background_color: None,
            text: String::new(),
            id: String::new(),
            pressed_style: PressedStyle::default(),
            keybind: None,
            decoration_left: None,
            decoration_right: None,
        }
    }
}
/// Represents the behavior of a button once hovered/pressed
pub enum PressedStyle {
    /// Flips the color of the button
    /// Works best when the button has both a fg & bg color set
    Flip,
}
impl Default for PressedStyle {
    fn default() -> Self {
        Self::Flip
    }
}
impl UI for Button {
    fn render_and_process(
        &mut self,
        start: (i32, i32),
        _end: (i32, i32),
        window: &mut Window,
        keyboard: &NiceKeyboard,
        data: &mut UIData,
    ) {
        //_
        let len = self.text.len(); //>=//x//y
                                   //&
        let hovered = keyboard.mouse_position.0 >= start.0
            && keyboard.mouse_position.1 == start.1
            && keyboard.mouse_position.0 < start.0 + len as i32
            || (data.selected.as_ref()).map_or(false, |f| f.eq(&self.id)); //);//&
                                                                           //if {

        //}
        let pressed = keyboard.mouse_pressed
            || data
                .config
                .key_select
                .as_ref()
                .map_or(false, |f| keyboard.keys.contains(f)); //false//default//f
        let highlight = hovered; //pressed||
                                 /* if hovered&&pressed{//highlight
                                 println!("press!");
                                 }// */
        if let Some(m) = data.data.get(&self.id) {
            match m {
                UIDataEntry::Text(_) => {
                    // todo!()
                    // wat
                }
                UIDataEntry::Boolean(d) => {
                    //todo!()
                    if *d && !(hovered && pressed) {
                        data.event(Event::Pressed(self.id.clone()));
                    } else if !d && hovered && pressed {
                        data.event(Event::Unpressed(self.id.clone()));
                    }
                } //_
            }
        }
        data.data
            .insert(self.id.clone(), UIDataEntry::Boolean(hovered && pressed)); //
        if let Some(n) = &self.decoration_left {
            let pos = (start.0, start.1);
            if let Some(k) = n.bg {
                window.set_bg_at(pos.0, pos.1, k);
            }
            if let Some(character) = n.ch {
                window.set_char_at(pos.0, pos.1, character);
            }
            if let Some(f) = n.fg {
                window.set_fg_at(pos.0, pos.1, f);
            }
        }
        if let Some(n) = &self.decoration_right {
            //left//+1
            let pos = (
                start.0
                    + if self.decoration_left.is_some() { 1 } else { 0 }
                    + self.text.len() as i32,
                start.1,
            );
            if let Some(k) = n.bg {
                window.set_bg_at(pos.0, pos.1, k);
            }
            if let Some(character) = n.ch {
                window.set_char_at(pos.0, pos.1, character);
            }
            if let Some(f) = n.fg {
                window.set_fg_at(pos.0, pos.1, f);
            }
        }
        match self.pressed_style {
            PressedStyle::Flip => {
                //todo!()
                if highlight {
                    //ç

                    window.print_at(
                        start.0
                            + if let Some(_k) = &self.decoration_left {
                                1
                            } else {
                                0
                            },
                        start.1,
                        &self.text,
                        self.background_color,
                        self.foreground_color,
                    ); //todo!()//window.
                } else {
                    // the original
                    window.print_at(
                        start.0,
                        start.1
                            + if let Some(_k) = &self.decoration_left {
                                1
                            } else {
                                0
                            },
                        &self.text,
                        self.foreground_color,
                        self.background_color,
                    ); //todo!()//window.
                }
            }
        }
    }
}
/// A single decorated char
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SingleCharDecoration {
    pub fg: Option<Col>,
    pub bg: Option<Col>,
    pub ch: Option<char>,
}
#[derive(Debug)]
/// This enum represents different events you can read from the UI's data
pub enum Event {
    //struct
    Pressed(ID),
    Unpressed(ID),
}
/// UI Configuration struct
/// Describes the behaviour, mainly, of keyboard (classic) interaction vs mouse (modern)
pub struct UIConfig {
    /// The accept key, default to enter
    pub key_select: Option<crate::TheKeyTypeFromWinit>,
    /// Defaults to DOWN (ArrowDown)
    pub key_down: Option<crate::TheKeyTypeFromWinit>,
    /// Defaults to UP (ArrowUp)
    pub key_up: Option<crate::TheKeyTypeFromWinit>,
    /// Defaults to exit
    pub key_exit: Option<crate::TheKeyTypeFromWinit>,
    /// Defaults to true
    pub uses_mouse: bool,
}
impl Default for UIConfig {
    fn default() -> Self {
        Self {
            key_select: Some(crate::TheKeyTypeFromWinit::Code(
                crate::TheKeyCodeTypeFromWinit::Enter,
            )), //Enter//select
            key_down: Some(crate::TheKeyTypeFromWinit::Code(
                crate::TheKeyCodeTypeFromWinit::ArrowDown,
            )),
            key_up: Some(crate::TheKeyTypeFromWinit::Code(
                crate::TheKeyCodeTypeFromWinit::ArrowUp,
            )),
            key_exit: Some(crate::TheKeyTypeFromWinit::Code(
                crate::TheKeyCodeTypeFromWinit::Escape,
            )),
            uses_mouse: true,
        } // todo!()
    }
}
/// is implemented for () to act as an empty element which does nothing
impl UI for () {
    fn render_and_process(
        &mut self,
        _start: (i32, i32),
        _end: (i32, i32),
        _window: &mut Window,
        _keyboard: &NiceKeyboard,
        _data: &mut UIData,
    ) //{
    {
    }
}

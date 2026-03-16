use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use velox::prelude::*;
use velox::scene::Color;
use velox::ui::{self, ClickEvent, FontWeight, InputHandle, IntoAnyElement};

const SIDEBAR_WIDTH: f32 = 260.0;
const AVATAR_SIZE: f32 = 40.0;
const MSG_AVATAR_SIZE: f32 = 32.0;
const CHAT_ITEM_HEIGHT: f32 = 72.0;
const BUBBLE_MAX_WIDTH: f32 = 360.0;
const BUBBLE_PADDING_H: f32 = 12.0;
const BUBBLE_PADDING_V: f32 = 8.0;
const MESSAGE_SPACING: f32 = 8.0;

const SIDEBAR_BG: Color = Color {
    r: 245,
    g: 245,
    b: 245,
    a: 255,
};
const SIDEBAR_BORDER: Color = Color {
    r: 220,
    g: 220,
    b: 220,
    a: 255,
};
const SELECTED_BG: Color = Color {
    r: 58,
    g: 122,
    b: 254,
    a: 255,
};
const HOVER_BG: Color = Color {
    r: 232,
    g: 232,
    b: 235,
    a: 255,
};
const BUBBLE_INCOMING: Color = Color {
    r: 233,
    g: 233,
    b: 235,
    a: 255,
};
const BUBBLE_OUTGOING: Color = Color {
    r: 58,
    g: 122,
    b: 254,
    a: 255,
};
const TEXT_PRIMARY: Color = Color {
    r: 30,
    g: 30,
    b: 30,
    a: 255,
};
const TEXT_SECONDARY: Color = Color {
    r: 130,
    g: 130,
    b: 130,
    a: 255,
};
const TEXT_WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
const HEADER_BORDER: Color = Color {
    r: 230,
    g: 230,
    b: 230,
    a: 255,
};
const INPUT_FIELD_BG: Color = Color {
    r: 235,
    g: 235,
    b: 237,
    a: 255,
};
const SEARCH_BG: Color = Color {
    r: 235,
    g: 235,
    b: 237,
    a: 255,
};
const MSG_AVATAR_BG: Color = Color {
    r: 200,
    g: 200,
    b: 205,
    a: 255,
};
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
const TRANSPARENT: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

const AVATAR_COLORS: [Color; 4] = [
    Color {
        r: 58,
        g: 122,
        b: 254,
        a: 255,
    },
    Color {
        r: 76,
        g: 175,
        b: 80,
        a: 255,
    },
    Color {
        r: 255,
        g: 152,
        b: 0,
        a: 255,
    },
    Color {
        r: 156,
        g: 39,
        b: 176,
        a: 255,
    },
];

#[derive(Clone)]
struct Message {
    text: String,
    is_outgoing: bool,
}

#[derive(Clone)]
struct Contact {
    name: String,
    last_message: String,
    timestamp: String,
    avatar_color_idx: usize,
}

struct ChatState {
    contacts: Vec<Contact>,
    conversations: Vec<Vec<Message>>,
    selected_contact: usize,
    input_handle: InputHandle,
}

impl ChatState {
    fn new() -> Self {
        Self {
            contacts: vec![
                Contact {
                    name: "Alice Smith".into(),
                    last_message: "Sounds good. See ya!".into(),
                    timestamp: "10:40 AM".into(),
                    avatar_color_idx: 0,
                },
                Contact {
                    name: "Bob Jones".into(),
                    last_message: "You: Yes, just emailed it to you.".into(),
                    timestamp: "Yesterday".into(),
                    avatar_color_idx: 1,
                },
                Contact {
                    name: "Charlie Brown".into(),
                    last_message: "You: Will do, thanks!".into(),
                    timestamp: "Tuesday".into(),
                    avatar_color_idx: 2,
                },
                Contact {
                    name: "Diana Prince".into(),
                    last_message: "You: Thank you so much!".into(),
                    timestamp: "Monday".into(),
                    avatar_color_idx: 3,
                },
            ],
            conversations: vec![
                vec![
                    Message {
                        text: "Hey! Are we still on for tomorrow?".into(),
                        is_outgoing: false,
                    },
                    Message {
                        text: "Yes, absolutely! See you at 10.".into(),
                        is_outgoing: true,
                    },
                    Message {
                        text: "Perfect. Should I bring anything?".into(),
                        is_outgoing: false,
                    },
                    Message {
                        text: "Just yourself!".into(),
                        is_outgoing: true,
                    },
                    Message {
                        text: "Sounds good. See ya!".into(),
                        is_outgoing: false,
                    },
                ],
                vec![
                    Message {
                        text: "Can you send me the report?".into(),
                        is_outgoing: false,
                    },
                    Message {
                        text: "Yes, just emailed it to you.".into(),
                        is_outgoing: true,
                    },
                ],
                vec![
                    Message {
                        text: "Don't forget the meeting at 3.".into(),
                        is_outgoing: false,
                    },
                    Message {
                        text: "Will do, thanks!".into(),
                        is_outgoing: true,
                    },
                ],
                vec![
                    Message {
                        text: "Thanks for your help today!".into(),
                        is_outgoing: false,
                    },
                    Message {
                        text: "Thank you so much!".into(),
                        is_outgoing: true,
                    },
                ],
            ],
            selected_contact: 0,
            input_handle: InputHandle::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

struct Avatar {
    size: f32,
    color: Color,
}

impl Render for Avatar {
    fn render(self) -> ui::element::AnyElement {
        let size = self.size;
        let color = self.color;
        canvas(move |bounds, commands| {
            let cx = bounds.x + size / 2.0;
            let cy = bounds.y + size / 2.0;
            commands.fill_circle(cx, cy, size / 2.0, color);
        })
        .w(px(size))
        .h(px(size))
        .into_any_element()
    }
}

impl_component!(Avatar);

struct ContactItem {
    name: String,
    last_message: String,
    timestamp: String,
    avatar_color: Color,
    is_selected: bool,
    on_select: Arc<dyn Fn(&ClickEvent)>,
}

impl Render for ContactItem {
    fn render(self) -> ui::element::AnyElement {
        let (name_color, sub_color) = if self.is_selected {
            (TEXT_WHITE, Color::rgba(220, 220, 255, 255))
        } else {
            (TEXT_PRIMARY, TEXT_SECONDARY)
        };
        let bg = if self.is_selected {
            SELECTED_BG
        } else {
            TRANSPARENT
        };
        let selected = self.is_selected;

        div()
            .h(px(CHAT_ITEM_HEIGHT))
            .w(pct(100.0))
            .flex_row()
            .items_center()
            .p(px(8.0))
            .gap(px(12.0))
            .bg(bg)
            .rounded(px(10.0))
            .cursor_pointer()
            .hover(move |s| s.bg(if selected { SELECTED_BG } else { HOVER_BG }))
            .on_click({
                let cb = self.on_select;
                move |e: &ClickEvent| cb(e)
            })
            .child(Avatar {
                size: AVATAR_SIZE,
                color: self.avatar_color,
            })
            .child(
                div()
                    .flex_col()
                    .flex_1()
                    .gap(px(4.0))
                    .child(
                        div()
                            .flex_row()
                            .justify_between()
                            .child(text(self.name).text_color(name_color))
                            .child(text(self.timestamp).text_sm().text_color(sub_color)),
                    )
                    .child(text(self.last_message).text_sm().text_color(sub_color)),
            )
            .into_any_element()
    }
}

impl_component!(ContactItem);

struct SearchBar;

impl Render for SearchBar {
    fn render(self) -> ui::element::AnyElement {
        div()
            .h(px(44.0))
            .p(px(8.0))
            .child(
                div()
                    .h(px(30.0))
                    .w(pct(100.0))
                    .bg(SEARCH_BG)
                    .rounded(px(8.0))
                    .items_center()
                    .flex_row()
                    .px_pad(px(8.0))
                    .child(text("Search").text_sm().text_color(TEXT_SECONDARY)),
            )
            .into_any_element()
    }
}

impl_component!(SearchBar);

struct Sidebar {
    contacts: Vec<Contact>,
    selected: usize,
    state: Rc<RefCell<ChatState>>,
}

impl Render for Sidebar {
    fn render(self) -> ui::element::AnyElement {
        let mut contact_list = div()
            .flex_1()
            .flex_col()
            .overflow_y_scroll()
            .p(px(4.0))
            .gap(px(2.0));

        for (i, contact) in self.contacts.iter().enumerate() {
            let state = self.state.clone();
            contact_list = contact_list.child(ContactItem {
                name: contact.name.clone(),
                last_message: contact.last_message.clone(),
                timestamp: contact.timestamp.clone(),
                avatar_color: AVATAR_COLORS[contact.avatar_color_idx % AVATAR_COLORS.len()],
                is_selected: i == self.selected,
                on_select: Arc::new(move |_: &ClickEvent| {
                    state.borrow_mut().selected_contact = i;
                }),
            });
        }

        div()
            .w(px(SIDEBAR_WIDTH))
            .h(pct(100.0))
            .flex_col()
            .bg(SIDEBAR_BG)
            .border_r(px(1.0))
            .border_color(SIDEBAR_BORDER)
            .child(SearchBar)
            .child(contact_list)
            .into_any_element()
    }
}

impl_component!(Sidebar);

struct MessageBubble {
    text: String,
    is_outgoing: bool,
}

impl Render for MessageBubble {
    fn render(self) -> ui::element::AnyElement {
        let (bubble_color, text_color) = if self.is_outgoing {
            (BUBBLE_OUTGOING, TEXT_WHITE)
        } else {
            (BUBBLE_INCOMING, TEXT_PRIMARY)
        };

        let bubble = div()
            .max_w(px(BUBBLE_MAX_WIDTH))
            .bg(bubble_color)
            .rounded(px(18.0))
            .py(px(BUBBLE_PADDING_V))
            .px_pad(px(BUBBLE_PADDING_H))
            .child(text(self.text).text_color(text_color));

        let mut row = div().w(pct(100.0)).flex_row().items_center().gap(px(8.0));

        if self.is_outgoing {
            row = row.justify_end().child(bubble);
        } else {
            row = row
                .child(Avatar {
                    size: MSG_AVATAR_SIZE,
                    color: MSG_AVATAR_BG,
                })
                .child(bubble);
        }

        row.into_any_element()
    }
}

impl_component!(MessageBubble);

struct ChatHeader {
    name: String,
}

impl Render for ChatHeader {
    fn render(self) -> ui::element::AnyElement {
        div()
            .h(px(52.0))
            .w(pct(100.0))
            .flex_row()
            .items_center()
            .px_pad(px(20.0))
            .border_b(px(1.0))
            .border_color(HEADER_BORDER)
            .child(
                text(self.name)
                    .text_lg()
                    .font_weight(FontWeight::Semibold)
                    .text_color(TEXT_PRIMARY),
            )
            .into_any_element()
    }
}

impl_component!(ChatHeader);

struct InputBar {
    handle: InputHandle,
}

impl Render for InputBar {
    fn render(self) -> ui::element::AnyElement {
        div()
            .h(px(52.0))
            .w(pct(100.0))
            .flex_row()
            .items_center()
            .gap(px(12.0))
            .px_pad(px(16.0))
            .border_t(px(1.0))
            .border_color(HEADER_BORDER)
            .child(text("+").text_lg().text_color(TEXT_SECONDARY))
            .child(
                input()
                    .handle(self.handle)
                    .placeholder("iMessage")
                    .focusable()
                    .flex_1()
                    .h(px(34.0))
                    .bg(INPUT_FIELD_BG)
                    .rounded(px(17.0))
                    .px_pad(px(12.0)),
            )
            .into_any_element()
    }
}

impl_component!(InputBar);

struct ChatPanel {
    contact_name: String,
    messages: Vec<Message>,
    input_handle: InputHandle,
}

impl Render for ChatPanel {
    fn render(self) -> ui::element::AnyElement {
        let mut message_area = div()
            .flex_1()
            .flex_col()
            .overflow_y_scroll()
            .p(px(20.0))
            .gap(px(MESSAGE_SPACING));

        for msg in &self.messages {
            message_area = message_area.child(MessageBubble {
                text: msg.text.clone(),
                is_outgoing: msg.is_outgoing,
            });
        }

        div()
            .flex_1()
            .h(pct(100.0))
            .flex_col()
            .bg(WHITE)
            .child(ChatHeader {
                name: self.contact_name,
            })
            .child(message_area)
            .child(InputBar {
                handle: self.input_handle,
            })
            .into_any_element()
    }
}

impl_component!(ChatPanel);

struct ChatApp {
    state: Rc<RefCell<ChatState>>,
}

impl Render for ChatApp {
    fn render(self) -> ui::element::AnyElement {
        let s = self.state.borrow();
        let selected = s.selected_contact;
        let contacts = s.contacts.clone();
        let contact_name = s.contacts[selected].name.clone();
        let messages = s.conversations[selected].clone();
        let input_handle = s.input_handle.clone();
        drop(s);

        div()
            .size_full()
            .flex_row()
            .bg(WHITE)
            .child(Sidebar {
                contacts,
                selected,
                state: self.state.clone(),
            })
            .child(ChatPanel {
                contact_name,
                messages,
                input_handle,
            })
            .into_any_element()
    }
}

impl_component!(ChatApp);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Rc::new(RefCell::new(ChatState::new()));

    App::new()
        .name("Velox Chat")
        .window(WindowConfig::new("main").title("Velox Chat").size(860, 540))
        .continuous_redraw()
        .setup_ui(move || {
            vec![
                ChatApp {
                    state: state.clone(),
                }
                .into_any_element(),
            ]
        })
        .run()
}

use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

pub struct Protocol {
    pub name: String,
    pub msgs: Vec<MessageGroup>,
}

pub struct MessageGroup {
    pub name: String,
    pub client: Vec<Message>,
    pub server: Vec<Message>,
}

pub struct Message {
    pub id: u16,
    pub name: String,
    pub elem: Vec<MessageField>,
}

pub struct MessageField {
    pub name: String,
    pub r#type: String,
    pub type_name: Option<String>,
    pub size: usize,
    pub elem: Vec<MessageField>,
}

fn attr(e: &BytesStart<'_>, key: &[u8]) -> String {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .map(|a| String::from_utf8_lossy(&a.value).into_owned())
        .unwrap()
}

fn attr_opt(e: &BytesStart<'_>, key: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .map(|a| String::from_utf8_lossy(&a.value).into_owned())
}

fn attr_num(e: &BytesStart<'_>, key: &[u8]) -> usize {
    attr_opt(e, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or_default()
}

pub fn parse(xml: &str) -> Vec<Protocol> {
    let mut xml = Reader::from_str(xml);
    xml.config_mut().trim_text(true);

    let mut protocols = Vec::new();

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == b"Protocol" => {
                protocols.push(parse_protocol(&mut xml, &e));
            }
            Event::Eof => break,
            _ => {}
        }
    }

    protocols
}

fn parse_protocol(xml: &mut Reader<&[u8]>, e: &BytesStart<'_>) -> Protocol {
    let mut protocol = Protocol {
        name: attr(e, b"Name"),
        msgs: Vec::new(),
    };

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == b"Messages" => {
                protocol.msgs.push(parse_message_group(xml, &e));
            }
            Event::End(e) if e.name().as_ref() == b"Protocol" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    protocol
}

fn parse_message_group(xml: &mut Reader<&[u8]>, e: &BytesStart<'_>) -> MessageGroup {
    let mut message_group = MessageGroup {
        name: attr(e, b"Name"),
        client: Vec::new(),
        server: Vec::new(),
    };

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == b"Client" => {
                message_group.client = parse_messages(xml, b"Client");
            }
            Event::Start(e) if e.name().as_ref() == b"Server" => {
                message_group.server = parse_messages(xml, b"Server");
            }
            Event::End(e) if e.name().as_ref() == b"Messages" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    message_group
}

fn parse_messages(xml: &mut Reader<&[u8]>, end: &[u8]) -> Vec<Message> {
    let mut messages = Vec::new();

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == b"Message" => {
                messages.push(Message {
                    id: attr_num(&e, b"Id") as u16,
                    name: attr(&e, b"Name"),
                    elem: parse_message_fields(xml, b"Message"),
                });
            }
            Event::Empty(e) if e.name().as_ref() == b"Message" => {
                messages.push(Message {
                    id: attr_num(&e, b"Id") as u16,
                    name: attr(&e, b"Name"),
                    elem: Vec::new(),
                });
            }
            Event::End(e) if e.name().as_ref() == end => break,
            Event::Eof => break,
            _ => {}
        }
    }

    messages
}

fn parse_message_fields(xml: &mut Reader<&[u8]>, end: &[u8]) -> Vec<MessageField> {
    let mut message_fields = Vec::new();

    loop {
        match xml.read_event().unwrap() {
            Event::Empty(e) => message_fields.push(parse_message_field(&e, Vec::new())),
            Event::Start(e) => message_fields.push(parse_message_field(
                &e,
                parse_message_fields(xml, e.name().as_ref()),
            )),
            Event::End(e) if e.name().as_ref() == end => break,
            Event::Eof => break,
            _ => {}
        }
    }

    message_fields
}

fn parse_message_field(e: &BytesStart<'_>, elem: Vec<MessageField>) -> MessageField {
    MessageField {
        name: attr(e, b"Name"),
        r#type: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
        type_name: attr_opt(e, b"TypeName"),
        size: attr_num(e, b"Size"),
        elem,
    }
}

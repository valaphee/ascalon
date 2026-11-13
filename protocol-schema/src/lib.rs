use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

pub struct Protocol {
    pub name: String,
    pub msgs: Vec<Messages>,
}

pub struct Messages {
    pub name:   String,
    pub client: Vec<Message>,
    pub server: Vec<Message>,
}

pub struct Message {
    pub id:     u16,
    pub name:   String,
    pub fields: Vec<Field>,
}

pub struct Field {
    pub name:      String,
    pub r#type:    String,
    pub size:      usize,
    pub type_name: Option<String>,
    pub fields:    Vec<Field>,
}

pub fn parse(xml: &str) -> Vec<Protocol> {
    let mut reader = Reader::from_str(xml);
    let config = reader.config_mut();
    config.trim_text(true);
    config.expand_empty_elements = true;

    let mut protocols = Vec::new();

    loop {
        match reader.read_event().unwrap() {
            Event::Start(element) if element.name().into_inner() == "Protocol" => {
                protocols.push(parse_protocol(&mut reader, &element));
            }
            Event::Eof => break,
            _ => {}
        }
    }

    protocols
}

fn parse_protocol(reader: &mut Reader<&[u8]>, element: &BytesStart<'_>) -> Protocol {
    let mut protocol = Protocol {
        name: attr(element, "Name").unwrap(),
        msgs: Vec::new(),
    };

    loop {
        match reader.read_event().unwrap() {
            Event::Start(element) if element.name().into_inner() == "Messages" => {
                protocol.msgs.push(parse_messages(reader, &element));
            }
            Event::End(element) if element.name().into_inner() == "Protocol" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    protocol
}

fn parse_messages(reader: &mut Reader<&[u8]>, element: &BytesStart<'_>) -> Messages {
    let mut messages = Messages {
        name:   attr(element, "Name").unwrap(),
        client: Vec::new(),
        server: Vec::new(),
    };

    loop {
        match reader.read_event().unwrap() {
            Event::Start(element) => match element.name().into_inner() {
                "Client" => messages.client = parse_direction(reader, "Client"),
                "Server" => messages.server = parse_direction(reader, "Server"),
                _ => {}
            },
            Event::End(element) if element.name().into_inner() == "Messages" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    messages
}

fn parse_direction(reader: &mut Reader<&[u8]>, end: &str) -> Vec<Message> {
    let mut messages = Vec::new();

    loop {
        match reader.read_event().unwrap() {
            Event::Start(element) if element.name().into_inner() == "Message" => {
                messages.push(Message {
                    id:     attr(&element, "Id").map_or(0, |id| id.parse().unwrap()),
                    name:   attr(&element, "Name").unwrap(),
                    fields: parse_fields(reader, "Message"),
                });
            }
            Event::End(element) if element.name().into_inner() == end => break,
            Event::Eof => break,
            _ => {}
        }
    }

    messages
}

fn parse_fields(reader: &mut Reader<&[u8]>, end: &str) -> Vec<Field> {
    let mut fields = Vec::new();

    loop {
        match reader.read_event().unwrap() {
            Event::Start(element) => {
                let type_name = element.name().into_inner();

                fields.push(Field {
                    name:      attr(&element, "Name").unwrap(),
                    r#type:    type_name.to_owned(),
                    size:      attr(&element, "Size").map_or(0, |size| size.parse().unwrap()),
                    type_name: attr(&element, "TypeName"),
                    fields:    parse_fields(reader, type_name),
                });
            }
            Event::End(element) if element.name().into_inner() == end => break,
            Event::Eof => break,
            _ => {}
        }
    }

    fields
}

fn attr(element: &BytesStart<'_>, name: &str) -> Option<String> {
    element.try_get_attribute(name).unwrap().map(|attribute| {
        attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .unwrap()
            .into_owned()
    })
}

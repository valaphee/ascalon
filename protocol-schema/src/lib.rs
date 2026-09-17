use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

pub struct Protocol {
    pub name: String,
    pub msgs: Vec<Messages>,
}

pub struct Messages {
    pub name: String,
    pub client: Vec<Message>,
    pub server: Vec<Message>,
}

pub struct Message {
    pub id: u16,
    pub name: String,
    pub fields: Vec<Field>,
}

pub struct Field {
    pub name: String,
    pub r#type: String,
    pub size: usize,
    pub type_name: Option<String>,
    pub fields: Vec<Field>,
}

pub fn parse(xml: &str) -> Vec<Protocol> {
    let mut xml = Reader::from_str(xml);
    xml.config_mut().trim_text(true);

    let mut protocols = Vec::new();

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == "Protocol" => {
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
        name: e
            .try_get_attribute("Name")
            .unwrap()
            .unwrap()
            .value
            .into_owned(),
        msgs: Vec::new(),
    };

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == "Messages" => {
                protocol.msgs.push(parse_messages(xml, &e));
            }
            Event::End(e) if e.name().as_ref() == "Protocol" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    protocol
}

fn parse_messages(xml: &mut Reader<&[u8]>, e: &BytesStart<'_>) -> Messages {
    let mut messages = Messages {
        name: e
            .try_get_attribute("Name")
            .unwrap()
            .unwrap()
            .value
            .into_owned(),
        client: Vec::new(),
        server: Vec::new(),
    };

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == "Client" => {
                messages.client = parse_direction(xml, "Client");
            }
            Event::Start(e) if e.name().as_ref() == "Server" => {
                messages.server = parse_direction(xml, "Server");
            }
            Event::End(e) if e.name().as_ref() == "Messages" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    messages
}

fn parse_direction(xml: &mut Reader<&[u8]>, end: &str) -> Vec<Message> {
    let mut messages = Vec::new();

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == "Message" => {
                messages.push(Message {
                    id: e
                        .try_get_attribute("Id")
                        .unwrap()
                        .map_or_default(|a| a.value.into_owned().parse().unwrap()),
                    name: e
                        .try_get_attribute("Name")
                        .unwrap()
                        .unwrap()
                        .value
                        .into_owned(),
                    fields: parse_fields(xml, "Message"),
                });
            }
            Event::Empty(e) if e.name().as_ref() == "Message" => {
                messages.push(Message {
                    id: e
                        .try_get_attribute("Id")
                        .unwrap()
                        .map_or_default(|a| a.value.into_owned().parse().unwrap()),
                    name: e
                        .try_get_attribute("Name")
                        .unwrap()
                        .unwrap()
                        .value
                        .into_owned(),
                    fields: Vec::new(),
                });
            }
            Event::End(e) if e.name().as_ref() == end => break,
            Event::Eof => break,
            _ => {}
        }
    }

    messages
}

fn parse_fields(xml: &mut Reader<&[u8]>, end: &str) -> Vec<Field> {
    let mut fields = Vec::new();

    loop {
        match xml.read_event().unwrap() {
            Event::Empty(e) => fields.push(parse_field(&e, Vec::new())),
            Event::Start(e) => fields.push(parse_field(&e, parse_fields(xml, e.name().as_ref()))),
            Event::End(e) if e.name().as_ref() == end => break,
            Event::Eof => break,
            _ => {}
        }
    }

    fields
}

fn parse_field(e: &BytesStart<'_>, fields: Vec<Field>) -> Field {
    Field {
        name: e
            .try_get_attribute("Name")
            .unwrap()
            .unwrap()
            .value
            .into_owned(),
        r#type: e.name().as_ref().to_owned(),
        size: e
            .try_get_attribute("Size")
            .unwrap()
            .map_or_default(|a| a.value.into_owned().parse().unwrap()),
        type_name: e
            .try_get_attribute("TypeName")
            .unwrap()
            .map(|a| a.value.into_owned()),
        fields,
    }
}

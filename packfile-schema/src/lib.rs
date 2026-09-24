use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

pub struct Packfile {
    pub name: String,
    pub chunks: Vec<Chunk>,
}

pub struct Chunk {
    pub name: String,
    pub versions: Vec<Version>,
}

pub struct Version {
    pub id: u32,
    pub type_name: String,
    pub fields: Vec<Field>,
}

pub struct Field {
    pub name: String,
    pub r#type: String,
    pub size: usize,
    pub type_name: Option<String>,
    pub fields: Vec<Field>,
}

pub fn parse(xml: &str) -> Vec<Packfile> {
    let mut xml = Reader::from_str(xml);
    xml.config_mut().trim_text(true);

    let mut packfiles = Vec::new();

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == "Packfile" => {
                packfiles.push(parse_packfile(&mut xml, &e));
            }
            Event::Eof => break,
            _ => {}
        }
    }

    packfiles
}

fn parse_packfile(xml: &mut Reader<&[u8]>, e: &BytesStart<'_>) -> Packfile {
    let mut packfile = Packfile {
        name: attr(e, "Name").unwrap(),
        chunks: Vec::new(),
    };

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == "Chunk" => {
                packfile.chunks.push(parse_chunk(xml, &e));
            }
            Event::End(e) if e.name().as_ref() == "Packfile" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    packfile
}

fn parse_chunk(xml: &mut Reader<&[u8]>, e: &BytesStart<'_>) -> Chunk {
    let mut chunk = Chunk {
        name: attr(e, "Name").unwrap(),
        versions: Vec::new(),
    };

    loop {
        match xml.read_event().unwrap() {
            Event::Start(e) if e.name().as_ref() == "Version" => {
                chunk.versions.push(Version {
                    id: attr(&e, "Id").unwrap().parse().unwrap(),
                    type_name: attr(&e, "TypeName").unwrap(),
                    fields: parse_fields(xml, "Version"),
                });
            }
            Event::End(e) if e.name().as_ref() == "Chunk" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    chunk
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
        name: attr(e, "Name").unwrap(),
        r#type: e.name().as_ref().to_owned(),
        size: attr(e, "Size").map_or(0, |v| v.parse().unwrap()),
        type_name: attr(e, "TypeName"),
        fields,
    }
}

fn attr(e: &BytesStart<'_>, name: &str) -> Option<String> {
    e.try_get_attribute(name)
        .unwrap()
        .map(|a| a.value.into_owned())
}

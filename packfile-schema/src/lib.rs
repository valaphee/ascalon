use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

pub struct Packfile {
    pub name:   String,
    pub chunks: Vec<Chunk>,
}

pub struct Chunk {
    pub name:     String,
    pub versions: Vec<Version>,
}

pub struct Version {
    pub id:        u32,
    pub type_name: String,
    pub fields:    Vec<Field>,
}

pub struct Field {
    pub name:      String,
    pub r#type:    String,
    pub size:      usize,
    pub type_name: Option<String>,
    pub fields:    Vec<Field>,
}

pub fn parse(xml: &str) -> Vec<Packfile> {
    let mut reader = Reader::from_str(xml);
    let config = reader.config_mut();
    config.trim_text(true);
    config.expand_empty_elements = true;

    let mut packfiles = Vec::new();

    loop {
        match reader.read_event().unwrap() {
            Event::Start(element) if element.name().into_inner() == "Packfile" => {
                packfiles.push(parse_packfile(&mut reader, &element));
            }
            Event::Eof => break,
            _ => {}
        }
    }

    packfiles
}

fn parse_packfile(reader: &mut Reader<&[u8]>, element: &BytesStart<'_>) -> Packfile {
    let mut packfile = Packfile {
        name:   attr(element, "Name").unwrap(),
        chunks: Vec::new(),
    };

    loop {
        match reader.read_event().unwrap() {
            Event::Start(element) if element.name().into_inner() == "Chunk" => {
                packfile.chunks.push(parse_chunk(reader, &element));
            }
            Event::End(element) if element.name().into_inner() == "Packfile" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    packfile
}

fn parse_chunk(reader: &mut Reader<&[u8]>, element: &BytesStart<'_>) -> Chunk {
    let mut chunk = Chunk {
        name:     attr(element, "Name").unwrap(),
        versions: Vec::new(),
    };

    loop {
        match reader.read_event().unwrap() {
            Event::Start(element) if element.name().into_inner() == "Version" => {
                chunk.versions.push(Version {
                    id:        attr(&element, "Id").unwrap().parse().unwrap(),
                    type_name: attr(&element, "TypeName").unwrap(),
                    fields:    parse_fields(reader, "Version"),
                });
            }
            Event::End(element) if element.name().into_inner() == "Chunk" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    chunk
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

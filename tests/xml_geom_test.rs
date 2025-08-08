#[cfg(feature = "xml")]
use rayon::iter::ParallelIterator;
#[cfg(feature = "xml")]
use skyway::chunks::ChunkBuilder;
#[cfg(feature = "xml")]
use skyway::elements::{Element, ElementType, SimpleElementType};
#[cfg(feature = "xml")]
use skyway::readers::{Reader, XmlReader};
#[cfg(feature = "xml")]
use std::io::Write;
#[cfg(feature = "xml")]
use std::sync::mpsc;
#[cfg(feature = "xml")]
use tempfile::NamedTempFile;

#[cfg(feature = "xml")]
#[test]
fn test_geom_output_way_with_nodes() {
    let xml_data = r#"<?xml version="1.0" encoding="UTF-8"?>
<osm version="0.6" generator="Overpass API 0.7.62.7 375dc00a">
<note>The data included in this document is from www.openstreetmap.org. The data is made available under ODbL.</note>
<meta osm_base="2025-08-07T09:01:54Z"/>

  <way id="6035806">
    <bounds minlat="37.5782185" minlon="-77.5094234" maxlat="37.5790117" maxlon="-77.5087874"/>
    <nd ref="49620337" lat="37.5790117" lon="-77.5087874"/>
    <nd ref="49620335" lat="37.5785541" lon="-77.5091196"/>
    <nd ref="49613364" lat="37.5782185" lon="-77.5094234"/>
    <tag k="highway" v="residential"/>
    <tag k="name" v="Parrish Street"/>
  </way>

</osm>"#;

    let reader = XmlReader::new(true); // Enable rebuild_geometries
    let (metadata_tx, _metadata_rx) = mpsc::channel();
    let chunk_builder = ChunkBuilder::new(8192);

    // Write test data to a temporary file
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", xml_data).unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let chunks: Vec<_> = reader
        .read_file(Some(temp_path), metadata_tx, chunk_builder)
        .collect();

    // Should have one chunk with 4 elements (1 way + 3 fake nodes)
    assert_eq!(chunks.len(), 1);
    let elements = &chunks[0].content;
    assert_eq!(elements.len(), 4);

    // Check that we have 3 fake nodes with negative IDs
    let fake_nodes: Vec<&Element> = elements
        .iter()
        .filter(|e| matches!(e.element_type, ElementType::Node { .. }) && e.id < 0)
        .collect();
    assert_eq!(fake_nodes.len(), 3);

    // Check that the fake nodes have the correct coordinates
    let expected_coords = vec![
        (375790117, -775087874),
        (375785541, -775091196),
        (375782185, -775094234),
    ];

    for (i, node) in fake_nodes.iter().enumerate() {
        if let ElementType::Node { lat, lon } = node.element_type {
            assert_eq!((lat, lon), expected_coords[i]);
        }
    }

    // Check the way element
    let way = elements
        .iter()
        .find(|e| matches!(e.element_type, ElementType::Way { .. }))
        .expect("Should have a way element");

    assert_eq!(way.id, 6035806);
    if let ElementType::Way { ref nodes } = way.element_type {
        assert_eq!(nodes.len(), 3);
        // All node refs should be negative (fake nodes)
        assert!(nodes.iter().all(|&id| id < 0));
    }
}

#[cfg(feature = "xml")]
#[test]
fn test_geom_output_relation_with_members() {
    let xml_data = r#"<?xml version="1.0" encoding="UTF-8"?>
<osm version="0.6" generator="Overpass API 0.7.62.7 375dc00a">
<note>The data included in this document is from www.openstreetmap.org. The data is made available under ODbL.</note>
<meta osm_base="2025-08-07T09:01:54Z"/>

  <relation id="19397249">
    <bounds minlat="37.5787480" minlon="-77.5107619" maxlat="37.5798305" maxlon="-77.5090365"/>
    <member type="node" ref="2592301390" role="label" lat="37.5385087" lon="-77.4342800"/>
    <member type="way" ref="1417939819" role="outer">
      <nd lat="37.5796267" lon="-77.5098825"/>
      <nd lat="37.5793795" lon="-77.5100682"/>
      <nd lat="37.5794516" lon="-77.5102211"/>
    </member>
    <tag k="leisure" v="park"/>
    <tag k="name" v="Westwood Playground"/>
    <tag k="type" v="multipolygon"/>
  </relation>

</osm>"#;

    let reader = XmlReader::new(true); // Enable rebuild_geometries
    let (metadata_tx, _metadata_rx) = mpsc::channel();
    let chunk_builder = ChunkBuilder::new(8192);

    // Write test data to a temporary file
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", xml_data).unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let chunks: Vec<_> = reader
        .read_file(Some(temp_path), metadata_tx, chunk_builder)
        .collect();

    // Should have one chunk with 5 elements (1 relation + 1 fake node for member + 3 fake nodes for way member)
    assert_eq!(chunks.len(), 1);
    let elements = &chunks[0].content;
    assert_eq!(elements.len(), 5);

    // Check that we have 4 fake nodes with negative IDs
    let fake_nodes: Vec<&Element> = elements
        .iter()
        .filter(|e| matches!(e.element_type, ElementType::Node { .. }) && e.id < 0)
        .collect();
    assert_eq!(fake_nodes.len(), 4);

    // Check the relation element
    let relation = elements
        .iter()
        .find(|e| matches!(e.element_type, ElementType::Relation { .. }))
        .expect("Should have a relation element");

    assert_eq!(relation.id, 19397249);
    if let ElementType::Relation { ref members } = relation.element_type {
        assert_eq!(members.len(), 2);

        // First member should be a node with negative (fake) ID
        assert_eq!(members[0].t, Some(SimpleElementType::Node));
        assert!(members[0].id < 0);
        assert_eq!(members[0].role, Some("label".to_string()));

        // Second member should be a way with positive (real) ID
        assert_eq!(members[1].t, Some(SimpleElementType::Way));
        assert_eq!(members[1].id, 1417939819);
        assert_eq!(members[1].role, Some("outer".to_string()));
    }
}

#[cfg(feature = "xml")]
#[test]
fn test_geom_output_disabled() {
    let xml_data = r#"<?xml version="1.0" encoding="UTF-8"?>
<osm version="0.6" generator="Overpass API 0.7.62.7 375dc00a">
  <way id="6035806">
    <nd ref="49620337" lat="37.5790117" lon="-77.5087874"/>
    <nd ref="49620335" lat="37.5785541" lon="-77.5091196"/>
    <tag k="highway" v="residential"/>
  </way>
</osm>"#;

    let reader = XmlReader::new(false); // Disable rebuild_geometries
    let (metadata_tx, _metadata_rx) = mpsc::channel();
    let chunk_builder = ChunkBuilder::new(8192);

    // Write test data to a temporary file
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", xml_data).unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let chunks: Vec<_> = reader
        .read_file(Some(temp_path), metadata_tx, chunk_builder)
        .collect();

    // Should have one chunk with only 1 element (the way, no fake nodes)
    assert_eq!(chunks.len(), 1);
    let elements = &chunks[0].content;
    assert_eq!(elements.len(), 1);

    // Check the way element
    let way = &elements[0];
    assert_eq!(way.id, 6035806);
    if let ElementType::Way { ref nodes } = way.element_type {
        assert_eq!(nodes.len(), 2);
        // Node refs should be positive (real IDs)
        assert_eq!(nodes[0], 49620337);
        assert_eq!(nodes[1], 49620335);
    }
}

#[cfg(feature = "xml")]
#[test]
fn test_normal_osm_xml_still_works() {
    let xml_data = r#"<?xml version="1.0" encoding="UTF-8"?>
<osm version="0.6" generator="test">
  <node id="7053784733" lat="37.5794831" lon="-77.5107619"/>
  <node id="7053784734" lat="37.5787480" lon="-77.5093787"/>
  <way id="755100524">
    <nd ref="7053784733"/>
    <nd ref="7053784734"/>
    <tag k="highway" v="path"/>
  </way>
</osm>"#;

    let reader = XmlReader::new(true); // Enable rebuild_geometries (shouldn't affect normal XML)
    let (metadata_tx, _metadata_rx) = mpsc::channel();
    let chunk_builder = ChunkBuilder::new(8192);

    // Write test data to a temporary file
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", xml_data).unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let chunks: Vec<_> = reader
        .read_file(Some(temp_path), metadata_tx, chunk_builder)
        .collect();

    // Should have one chunk with 3 elements (2 nodes + 1 way)
    assert_eq!(chunks.len(), 1);
    let elements = &chunks[0].content;
    assert_eq!(elements.len(), 3);

    // All IDs should be positive
    assert!(elements.iter().all(|e| e.id > 0));

    // Check the way references the correct nodes
    let way = elements
        .iter()
        .find(|e| matches!(e.element_type, ElementType::Way { .. }))
        .expect("Should have a way element");

    if let ElementType::Way { ref nodes } = way.element_type {
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0], 7053784733);
        assert_eq!(nodes[1], 7053784734);
    }
}

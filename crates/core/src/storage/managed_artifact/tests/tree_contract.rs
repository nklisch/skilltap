#[test]
fn artifact_trees_validate_and_sort_complete_relative_files() {
    let tree = ArtifactTree::new([
        ("z/file", vec![3]),
        ("SKILL.md", vec![1]),
        ("a/nested/file", vec![2]),
    ])
    .unwrap();
    assert_eq!(
        tree.files()
            .keys()
            .map(RelativeArtifactPath::as_str)
            .collect::<Vec<_>>(),
        ["SKILL.md", "a/nested/file", "z/file"]
    );
    assert!(matches!(
        ArtifactTree::new(Vec::<(String, Vec<u8>)>::new()),
        Err(ArtifactTreeError::Empty)
    ));
    for invalid in ["", "/absolute", "../escape", "a/../b", "a//b", "./a"] {
        assert!(matches!(
            ArtifactTree::new([(invalid, Vec::new())]),
            Err(ArtifactTreeError::InvalidPath)
        ));
    }
    assert!(matches!(
        ArtifactTree::new([("same", vec![1]), ("same", vec![2])]),
        Err(ArtifactTreeError::DuplicatePath { .. })
    ));
    assert!(matches!(
        ArtifactTree::new([("file", vec![1]), ("file/child", vec![2])]),
        Err(ArtifactTreeError::FileIsAncestor { .. })
    ));
}

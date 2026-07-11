#[test]
fn typed_replacements_create_root_once_per_call_and_are_byte_stable() {
    let filesystem = FakeFileSystem::default();
    let config = FileConfigRepository::new(&filesystem, root()).unwrap();
    let inventory = FileInventoryRepository::new(&filesystem, root()).unwrap();
    let state = FileStateRepository::new(&filesystem, root()).unwrap();
    let config_value = ConfigDocument::defaults();
    let inventory_value = empty_inventory();
    let state_value = empty_state();

    config.replace(&config_value).unwrap();
    inventory.replace(&inventory_value).unwrap();
    state.replace(&state_value).unwrap();
    let first = [
        filesystem.bytes(&path("config.toml")).unwrap(),
        filesystem.bytes(&path("inventory.toml")).unwrap(),
        filesystem.bytes(&path("state.json")).unwrap(),
    ];
    config.replace(&config_value).unwrap();
    inventory.replace(&inventory_value).unwrap();
    state.replace(&state_value).unwrap();
    let second = [
        filesystem.bytes(&path("config.toml")).unwrap(),
        filesystem.bytes(&path("inventory.toml")).unwrap(),
        filesystem.bytes(&path("state.json")).unwrap(),
    ];

    assert_eq!(first, second);
    assert_eq!(first[0], include_bytes!("../../fixtures/config.toml"));
    assert_eq!(config.load().unwrap(), DocumentState::Present(config_value));
    assert_eq!(
        inventory.load().unwrap(),
        DocumentState::Present(inventory_value)
    );
    assert_eq!(state.load().unwrap(), DocumentState::Present(state_value));
    assert_eq!(filesystem.created.borrow().len(), 6);
    assert_eq!(filesystem.writes.borrow().len(), 6);
}

#[test]
fn malformed_invalid_and_unsupported_documents_are_contextual_and_never_rewritten() {
    let filesystem = FakeFileSystem::default();
    let config_path = path("config.toml");
    let repository = FileConfigRepository::new(&filesystem, root()).unwrap();

    for (contents, action, failure) in [
        (
            b"secret-value = [".as_slice(),
            DocumentAction::Decode,
            StorageFailure::Malformed,
        ),
        (
            b"schema = 1\nunknown = \"secret-value\"\n".as_slice(),
            DocumentAction::Validate,
            StorageFailure::Invalid,
        ),
        (
            b"schema = 77\n".as_slice(),
            DocumentAction::Validate,
            StorageFailure::UnsupportedSchema { version: 77 },
        ),
    ] {
        filesystem.put(config_path.clone(), contents);
        let error = repository.load().unwrap_err();
        assert_eq!(error.document(), DocumentKind::Config);
        assert_eq!(error.action(), action);
        assert_eq!(error.path(), &config_path);
        assert_eq!(error.failure(), failure);
        assert!(!error.to_string().contains("secret-value"));
        assert!(!format!("{error:?}").contains("secret-value"));
    }
    assert!(filesystem.writes.borrow().is_empty());
}

#[test]
fn state_json_and_inventory_toml_keep_their_own_codec_context() {
    let filesystem = FakeFileSystem::default();
    filesystem.put(path("inventory.toml"), b"schema = 3\n".to_vec());
    filesystem.put(path("state.json"), br#"{"schema":4}"#.to_vec());

    let inventory = FileInventoryRepository::new(&filesystem, root())
        .unwrap()
        .load()
        .unwrap_err();
    let state = FileStateRepository::new(&filesystem, root())
        .unwrap()
        .load()
        .unwrap_err();
    assert_eq!(
        inventory.failure(),
        StorageFailure::UnsupportedSchema { version: 3 }
    );
    assert_eq!(inventory.document(), DocumentKind::Inventory);
    assert_eq!(
        state.failure(),
        StorageFailure::UnsupportedSchema { version: 4 }
    );
    assert_eq!(state.document(), DocumentKind::State);
    assert!(filesystem.writes.borrow().is_empty());

    for duplicate in [
        br#"{"schema":77,"schema":1,"harnesses":[],"resources":[]}"#.as_slice(),
        br#"{"schema":1,"schema":77,"harnesses":[],"resources":[]}"#.as_slice(),
        br#"{"schema":77,"harnesses":[],"harnesses":[],"resources":[]}"#.as_slice(),
    ] {
        filesystem.put(path("state.json"), duplicate);
        let error = FileStateRepository::new(&filesystem, root())
            .unwrap()
            .load()
            .unwrap_err();
        assert_eq!(error.action(), DocumentAction::Validate);
        assert_eq!(error.failure(), StorageFailure::Invalid);
    }

    filesystem.put(path("state.json"), br#"{"schema":1,"harnesses":["#.to_vec());
    let malformed = FileStateRepository::new(&filesystem, root())
        .unwrap()
        .load()
        .unwrap_err();
    assert_eq!(malformed.action(), DocumentAction::Decode);
    assert_eq!(malformed.failure(), StorageFailure::Malformed);
}

#[test]
fn hypothetical_config_version_does_not_change_inventory_or_state_contracts() {
    let config = ConfigDocument::defaults();
    let inventory = empty_inventory();
    let state = empty_state();
    let config_bytes = include_bytes!("../../fixtures/config.toml");
    let inventory_bytes = toml::to_string_pretty(&inventory).unwrap().into_bytes();
    let state_bytes = (serde_json::to_string_pretty(&state).unwrap() + "\n").into_bytes();
    let future_config = TomlCodec::new(CONFIG_SCHEMA_VERSION + 1);
    let current_inventory = TomlCodec::new(INVENTORY_SCHEMA_VERSION);
    let current_state = JsonCodec::new(STATE_SCHEMA_VERSION);

    assert!(matches!(
        <TomlCodec as DocumentCodec<ConfigDocument>>::decode(&future_config, config_bytes),
        Err(CodecFailure::UnsupportedSchema {
            version: CONFIG_SCHEMA_VERSION
        })
    ));
    assert!(matches!(
        <TomlCodec as DocumentCodec<InventoryDocument>>::decode(
            &current_inventory,
            &inventory_bytes
        ),
        Ok(document) if document == inventory
    ));
    assert!(matches!(
        <JsonCodec as DocumentCodec<StateDocument>>::decode(&current_state, &state_bytes),
        Ok(document) if document == state
    ));

    let Ok(encoded_config) =
        <TomlCodec as DocumentCodec<ConfigDocument>>::encode(&future_config, &config)
    else {
        panic!("valid config must encode")
    };
    let Ok(encoded_inventory) =
        <TomlCodec as DocumentCodec<InventoryDocument>>::encode(&current_inventory, &inventory)
    else {
        panic!("valid inventory must encode")
    };
    let Ok(encoded_state) =
        <JsonCodec as DocumentCodec<StateDocument>>::encode(&current_state, &state)
    else {
        panic!("valid state must encode")
    };
    assert_eq!(encoded_config, config_bytes);
    assert_eq!(encoded_inventory, inventory_bytes);
    assert_eq!(encoded_state, state_bytes);
}

mod api;
pub mod export;
pub mod file;
pub mod import;
mod model;
mod resolve;
mod store;

pub use model::*;
pub use resolve::*;
pub use store::*;

#[cfg(test)]
mod tests {
    use super::*;
    use super::store::now_ms;
    use uuid::Uuid;

    #[test]
    fn test_dynamic_timestamp_and_guid() {
        let text = "postman-{{$timestamp}}";
        let result = interpolate_dynamic_variables(text);
        assert!(result.starts_with("postman-"));
        let ts_str = result.trim_start_matches("postman-");
        assert!(ts_str.parse::<u64>().is_ok());

        let guid_text = "UUID: {{$guid}}";
        let guid_result = interpolate_dynamic_variables(guid_text);
        assert!(guid_result.starts_with("UUID: "));
        let uuid_str = guid_result.trim_start_matches("UUID: ");
        assert_eq!(uuid_str.len(), 36);
    }

    #[test]
    fn test_uuidv4_and_uuidv7() {
        let v4 = interpolate_dynamic_variables("{{$uuid}}");
        assert_eq!(v4.len(), 36);
        let parsed_v4 = Uuid::parse_str(&v4).expect("valid uuid");
        assert_eq!(parsed_v4.get_version_num(), 4);

        let v7 = interpolate_dynamic_variables("{{$uuidv7}}");
        assert_eq!(v7.len(), 36);
        let parsed_v7 = Uuid::parse_str(&v7).expect("valid uuid v7");
        assert_eq!(parsed_v7.get_version_num(), 7);
    }

    fn env_with(vars: &[(&str, &str)]) -> Environment {
        Environment {
            id: "env-totp".to_string(),
            workspace_id: "ws-1".to_string(),
            name: "Totp".to_string(),
            is_active: true,
            variables: vars
                .iter()
                .map(|(k, v)| EnvironmentVariable {
                    key: k.to_string(),
                    value: v.to_string(),
                    enabled: true,
                    r#type: "secret".to_string(),
                    active_index: 0,
                    variants: vec![],
                })
                .collect(),
            created_at_ms: 0,
            updated_at_ms: 0,
        }
    }

    fn expected_code(secret: &str) -> [String; 2] {
        let p = crate::utils::totp::parse_secret(secret).unwrap();
        let now = crate::utils::totp::now_unix();
        [crate::utils::totp::code_for_send(&p, now), crate::utils::totp::code_for_send(&p, now + 1)]
    }

    #[test]
    fn totp_ref_resolves_base32_and_otpauth_secrets() {
        let base32 = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        let uri = "otpauth://totp/x?secret=MZXW6&digits=8";
        let env = env_with(&[("adminTotpSecret", base32), ("uriSecret", uri)]);

        let code = interpolate_variables_with_env(&env, r#"{"code": "{{$totp:adminTotpSecret}}"}"#);
        let code = code.trim_start_matches(r#"{"code": ""#).trim_end_matches(r#""}"#);
        assert!(expected_code(base32).iter().any(|c| c == code), "{code}");

        let code = interpolate_variables_with_env(&env, "{{ $TOTP:URISECRET }}");
        assert_eq!(code.len(), 8);
        assert!(expected_code(uri).contains(&code));
    }

    #[test]
    fn totp_ref_with_unknown_var_or_bad_secret_stays_unresolved() {
        let env = env_with(&[("bad", "not base32!")]);
        assert_eq!(interpolate_variables_with_env(&env, "{{$totp:missing}}"), "{{$totp:missing}}");
        assert_eq!(interpolate_variables_with_env(&env, "{{$totp:bad}}"), "{{$totp:bad}}");
    }

    #[test]
    fn totp_ref_resolves_through_a_variable() {
        let env = env_with(&[("secret", "MZXW6"), ("code", "{{$totp:secret}}")]);
        let code = interpolate_variables_with_env(&env, "{{code}}");
        assert!(expected_code("MZXW6").contains(&code), "{code}");
        assert!(variable_value_in_env(Some(&env), "$totp:secret").is_some());
        assert!(totp_params_in_env(Some(&env), "$totp:secret").is_some());
    }

    #[test]
    fn cred_refs_resolve_directly_and_through_a_variant_link() {
        use crate::credentials::{create_credential, save_credential, Field, FieldKind};
        let db = crate::credentials::tests_support::temp_db();
        let ws = create_workspace_db(&db, "W".into(), None).unwrap().id;
        let mut cred = create_credential(&db, &crate::credentials::tests_support::vault(&db, &ws, "DG"), "dgAdmin").unwrap();
        cred.fields = vec![Field { key: "password".into(), kind: FieldKind::Secret, extracted: false, value: "hunter2".into() }];
        save_credential(&db, &cred).unwrap();

        let mut env = env_with(&[("adminPassword", "")]);
        env.variables[0].variants = vec![VariableVariant { name: "dev".into(), value: "{{$cred:DG/dgAdmin.password}}".into() }];
        let scope = Some(CredentialScope { db: &db, workspace_id: &ws });

        let mut record = Interpolation::default();
        let out = interpolate(Some(&env), scope, "p={{adminPassword}}&q={{ $cred:DG/DGADMIN.Password }}", &mut record);
        assert_eq!(out, "p=hunter2&q=hunter2");
        assert_eq!(record.secrets, ["hunter2"]);

        let missing = "{{$cred:DG/dgAdmin.nope}} {{$cred:DG/nobody.password}}";
        assert_eq!(interpolate(Some(&env), scope, missing, &mut Interpolation::default()), missing);
        assert_eq!(interpolate(None, None, "{{$cred:DG/dgAdmin.password}}", &mut record), "{{$cred:DG/dgAdmin.password}}");
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn mask_hides_raw_and_url_encoded_secrets() {
        let record = Interpolation { secrets: vec!["p@ss word".into()], locked: false };
        assert_eq!(record.mask("a=p@ss word&b=p%40ss%20word&c=p%40ss+word"), "a=••••&b=••••&c=••••");
        assert_eq!(Interpolation::default().mask("p@ss"), "p@ss");
    }

    #[test]
    fn test_nested_double_interpolation() {
        let env = Environment {
            id: "env-1".to_string(),
            workspace_id: "ws-1".to_string(),
            name: "Test Env".to_string(),
            is_active: true,
            variables: vec![
                EnvironmentVariable {
                    key: "var_a".to_string(),
                    value: "abc".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
                EnvironmentVariable {
                    key: "var_b".to_string(),
                    value: "{{var_a}}def".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        let result = interpolate_variables_with_env(&env, "https://api.com/{{var_b}}");
        assert_eq!(result, "https://api.com/abcdef");

        let result_spaces = interpolate_variables_with_env(&env, "https://api.com/{{ var_b }}");
        assert_eq!(result_spaces, "https://api.com/abcdef");

        let result_dynamic = interpolate_variables_with_env(&env, "X-Key: {{var_a}}-{{$timestamp}}");
        assert!(result_dynamic.starts_with("X-Key: abc-"));
    }

    #[test]
    fn test_deeply_nested_interpolation() {
        let env = Environment {
            id: "env-2".to_string(),
            workspace_id: "ws-1".to_string(),
            name: "Deep Env".to_string(),
            is_active: true,
            variables: vec![
                EnvironmentVariable {
                    key: "HOST".to_string(),
                    value: "example.com".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
                EnvironmentVariable {
                    key: "BASE_URL".to_string(),
                    value: "https://{{HOST}}/api".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
                EnvironmentVariable {
                    key: "USERS_ENDPOINT".to_string(),
                    value: "{{BASE_URL}}/v1/users".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        let result = interpolate_variables_with_env(&env, "{{USERS_ENDPOINT}}?page=1");
        assert_eq!(result, "https://example.com/api/v1/users?page=1");
    }

    #[test]
    fn test_circular_reference_safety() {
        let env = Environment {
            id: "env-3".to_string(),
            workspace_id: "ws-1".to_string(),
            name: "Circular Env".to_string(),
            is_active: true,
            variables: vec![
                EnvironmentVariable {
                    key: "a".to_string(),
                    value: "{{b}}".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
                EnvironmentVariable {
                    key: "b".to_string(),
                    value: "{{a}}".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        // Should not loop infinitely or panic
        let _ = interpolate_variables_with_env(&env, "{{a}}");
    }

    #[test]
    fn test_workspace_and_environment_db_crud() {
        let dir = std::env::temp_dir().join(format!("mitm-ws-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::new(dir.join("test.db"));
        crate::db::init_database(&db, None).unwrap();

        let ws = create_workspace_db(&db, "Test Workspace".to_string(), Some("Desc".to_string())).unwrap();
        assert_eq!(ws.name, "Test Workspace");
        assert_eq!(ws.description.as_deref(), Some("Desc"));

        let list = get_workspaces_db(&db).unwrap();
        assert!(list.iter().any(|w| w.id == ws.id));

        let fetched = get_workspace_db(&db, &ws.id).unwrap().unwrap();
        assert_eq!(fetched.id, ws.id);

        rename_workspace_db(&db, &ws.id, "Renamed WS").unwrap();
        update_workspace_description_db(&db, &ws.id, Some("New Desc")).unwrap();
        let updated = get_workspace_db(&db, &ws.id).unwrap().unwrap();
        assert_eq!(updated.name, "Renamed WS");
        assert_eq!(updated.description.as_deref(), Some("New Desc"));

        set_active_workspace_db(&db, &ws.id).unwrap();
        let active_ws = get_active_workspace_db(&db).unwrap().unwrap();
        assert_eq!(active_ws.id, ws.id);

        let env = Environment {
            id: Uuid::new_v4().to_string(),
            workspace_id: ws.id.clone(),
            name: "Dev".to_string(),
            is_active: true,
            variables: vec![EnvironmentVariable {
                key: "HOST".to_string(),
                value: "localhost".to_string(),
                enabled: true,
                r#type: "default".to_string(),
                active_index: 0,
                variants: vec![],
            }],
            created_at_ms: now_ms(),
            updated_at_ms: now_ms(),
        };
        save_workspace_environment_db(&db, env.clone()).unwrap();

        let envs = get_workspace_environments_db(&db, &ws.id).unwrap();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].name, "Dev");

        let fetched_env = get_workspace_environment_db(&db, &env.id).unwrap().unwrap();
        assert_eq!(fetched_env.name, "Dev");

        let dup = duplicate_workspace_environment_db(&db, &env.id, None).unwrap();
        assert_eq!(dup.name, "Dev (Copy)");
        assert!(!dup.is_active);

        set_active_environment_db(&db, &ws.id, Some(&dup.id)).unwrap();
        let active_env = get_workspace_environment_db(&db, &dup.id).unwrap().unwrap();
        assert!(active_env.is_active);
        let orig_env = get_workspace_environment_db(&db, &env.id).unwrap().unwrap();
        assert!(!orig_env.is_active);

        delete_workspace_environment_db(&db, &dup.id).unwrap();
        assert!(get_workspace_environment_db(&db, &dup.id).unwrap().is_none());

        delete_workspace_db(&db, &ws.id).unwrap();
        assert!(get_workspace_db(&db, &ws.id).unwrap().is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_vars_and_variable_value() {
        let env = Environment {
            id: "env-test".to_string(),
            workspace_id: "ws-test".to_string(),
            name: "Test Env".to_string(),
            is_active: true,
            variables: vec![
                EnvironmentVariable {
                    key: "HOST".to_string(),
                    value: "api.example.com".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
                EnvironmentVariable {
                    key: "REGION".to_string(),
                    value: "us-east-1".to_string(),
                    enabled: true,
                    r#type: "default".to_string(),
                    active_index: 1,
                    variants: vec![
                        VariableVariant { name: "us".to_string(), value: "us-east-1".to_string() },
                        VariableVariant { name: "eu".to_string(), value: "eu-west-1".to_string() },
                    ],
                },
                EnvironmentVariable {
                    key: "DISABLED_KEY".to_string(),
                    value: "secret".to_string(),
                    enabled: false,
                    r#type: "default".to_string(),
                    active_index: 0,
                    variants: vec![],
                },
            ],
            created_at_ms: 0,
            updated_at_ms: 0,
        };

        let template = "https://{{HOST}}/{{REGION}}/users/{{user_id}}?disabled={{DISABLED_KEY}}&uuid={{$guid}}";
        let (resolved, unresolved) = resolve_variables_with_env(Some(&env), template);

        assert!(resolved.starts_with("https://api.example.com/eu-west-1/users/{{user_id}}?disabled={{DISABLED_KEY}}&uuid="));
        assert_eq!(unresolved, vec![
            Unresolved { name: "user_id".to_string() },
            Unresolved { name: "DISABLED_KEY".to_string() },
        ]);

        assert_eq!(variable_value_in_env(Some(&env), "HOST"), Some("api.example.com".to_string()));
        assert_eq!(variable_value_in_env(Some(&env), "host"), Some("api.example.com".to_string()));
        assert_eq!(variable_value_in_env(Some(&env), "REGION"), Some("eu-west-1".to_string()));
        assert_eq!(variable_value_in_env(Some(&env), "DISABLED_KEY"), None);
        assert_eq!(variable_value_in_env(Some(&env), "user_id"), None);
        assert!(variable_value_in_env(Some(&env), "$guid").is_some());
    }

    #[test]
    fn test_active_environment_survives_restart() {
        let dir = std::env::temp_dir().join(format!("mitm-env-survive-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = crate::db::Db::new(dir.join("test.db"));
        crate::db::init_database(&db, None).unwrap();

        let ws = create_workspace_db(&db, "Main Workspace".to_string(), None).unwrap();
        set_active_workspace_db(&db, &ws.id).unwrap();

        let dev_env = Environment {
            id: "env-dev".to_string(),
            workspace_id: ws.id.clone(),
            name: "Dev".to_string(),
            is_active: true,
            variables: vec![EnvironmentVariable {
                key: "BASE_URL".to_string(),
                value: "https://dev.example.com".to_string(),
                enabled: true,
                r#type: "default".to_string(),
                active_index: 0,
                variants: vec![],
            }],
            created_at_ms: now_ms(),
            updated_at_ms: now_ms(),
        };
        save_workspace_environment_db(&db, dev_env).unwrap();

        let prod_env = Environment {
            id: "env-prod".to_string(),
            workspace_id: ws.id.clone(),
            name: "Prod".to_string(),
            is_active: false,
            variables: vec![EnvironmentVariable {
                key: "BASE_URL".to_string(),
                value: "https://prod.example.com".to_string(),
                enabled: true,
                r#type: "default".to_string(),
                active_index: 0,
                variants: vec![],
            }],
            created_at_ms: now_ms() + 1,
            updated_at_ms: now_ms() + 1,
        };
        save_workspace_environment_db(&db, prod_env).unwrap();

        set_active_environment_db(&db, &ws.id, Some("env-prod")).unwrap();

        let reloaded_ws = get_active_workspace_db(&db).unwrap().unwrap();
        assert_eq!(reloaded_ws.active_environment_id.as_deref(), Some("env-prod"));

        let envs = get_workspace_environments_db(&db, &reloaded_ws.id).unwrap();
        let active_env = envs.iter().find(|e| e.is_active).expect("active env found");
        assert_eq!(active_env.id, "env-prod");
        assert_eq!(active_env.name, "Prod");

        let resolved = interpolate_variables(&db, &reloaded_ws.id, "{{BASE_URL}}/api");
        assert_eq!(resolved, "https://prod.example.com/api");

        set_active_environment_db(&db, &ws.id, None).unwrap();
        let envs_none = get_workspace_environments_db(&db, &reloaded_ws.id).unwrap();
        assert!(envs_none.iter().all(|e| !e.is_active));

        let _ = std::fs::remove_dir_all(&dir);
    }
}


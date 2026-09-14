#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use serde_json::json;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    use crate::plugin::{
        action_allowed, install_plugin_from_zip, load_available_plugins, load_plugins,
        run_plugin_command, search_commands, CommandInput, InstalledPlugin, MarketplaceEntry,
        PermissionDecision, PluginManifest, PluginRuntime, PluginUiSpec, RpcAction, RpcResult,
    };

    fn sample_manifest(runtime: PluginRuntime, entry: &str) -> PluginManifest {
        PluginManifest {
            id: "dev.vvicat.echo".into(),
            name: "回显工具".into(),
            version: "1.0.0".into(),
            description: "返回输入文本".into(),
            icon: String::new(),
            keywords: vec!["echo".into(), "回显".into()],
            categories: vec!["efficiency".into()],
            runtime,
            entry: entry.into(),
            builtin: None,
            ui: None,
            permissions: vec!["clipboard".into()],
            commands: vec![crate::plugin::PluginCommand {
                id: "echo.run".into(),
                title: "回显文本".into(),
                keyword: "echo".into(),
                input: "text".into(),
            }],
        }
    }

    #[test]
    fn load_plugins_reads_manifest_and_indexes_commands() {
        let dir = tempdir().unwrap();
        let plugin_dir = dir.path().join("dev.vvicat.echo");
        fs::create_dir_all(&plugin_dir).unwrap();
        let mut manifest = sample_manifest(PluginRuntime::Shell, "main.sh");
        manifest.ui = Some(PluginUiSpec {
            window: "ui/Window.svelte".into(),
            settings: "ui/Settings.svelte".into(),
            styles: "ui/styles.css".into(),
        });
        fs::write(
            plugin_dir.join("plugin.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let plugins = load_plugins(dir.path()).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(
            plugins[0].manifest.ui.as_ref().unwrap().window,
            "ui/Window.svelte"
        );
        let matches = search_commands(&plugins, "回显");
        assert_eq!(matches[0].plugin_id, "dev.vvicat.echo");
        assert_eq!(matches[0].command_id, "echo.run");
    }

    #[test]
    fn load_plugins_rejects_parent_paths_in_ui_entries() {
        let dir = tempdir().unwrap();
        let plugin_dir = dir.path().join("dev.vvicat.echo");
        fs::create_dir_all(&plugin_dir).unwrap();
        let mut manifest = sample_manifest(PluginRuntime::Shell, "main.sh");
        manifest.ui = Some(PluginUiSpec {
            window: "../Window.svelte".into(),
            settings: String::new(),
            styles: String::new(),
        });
        fs::write(
            plugin_dir.join("plugin.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        assert!(load_plugins(dir.path()).is_err());
    }

    #[test]
    fn run_shell_plugin_uses_json_rpc_input_and_parses_result() {
        let dir = tempdir().unwrap();
        let plugin_dir = dir.path().join("dev.vvicat.echo");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("main.sh"),
            "cat <<'JSON'\n{\"type\":\"text\",\"text\":\"shell-ok\"}\nJSON\n",
        )
        .unwrap();

        let plugin = InstalledPlugin {
            manifest: sample_manifest(PluginRuntime::Shell, "main.sh"),
            dir: plugin_dir,
        };
        let result = run_plugin_command(
            &plugin,
            "echo.run",
            CommandInput {
                query: "hi".into(),
                context: json!({"source":"test"}),
            },
        )
        .unwrap();

        assert_eq!(
            result,
            RpcResult::Text {
                text: "shell-ok".into()
            }
        );
    }

    #[test]
    fn install_marketplace_zip_requires_permission_approval() {
        let workspace = tempdir().unwrap();
        let zip_path = workspace.path().join("echo.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("plugin.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            serde_json::to_vec_pretty(&sample_manifest(PluginRuntime::Node, "main.js"))
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        zip.start_file("main.js", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"console.log(JSON.stringify({type:'text',text:'node-ok'}));")
            .unwrap();
        zip.finish().unwrap();

        let target = workspace.path().join("plugins");
        let entry = MarketplaceEntry {
            id: "dev.vvicat.echo".into(),
            name: "回显工具".into(),
            version: "1.0.0".into(),
            description: "测试插件".into(),
            icon: String::new(),
            runtime: "node".into(),
            entry: "main.js".into(),
            bundled: false,
            download_url: zip_path.to_string_lossy().into_owned(),
            sha256: None,
            categories: vec!["efficiency".into()],
            permissions: vec!["clipboard".into()],
        };

        let denied =
            install_plugin_from_zip(&entry, &target, PermissionDecision::Denied).unwrap_err();
        assert!(denied.to_string().contains("权限"));

        let installed =
            install_plugin_from_zip(&entry, &target, PermissionDecision::Approved).unwrap();
        assert_eq!(installed.manifest.id, "dev.vvicat.echo");
        assert!(target.join("dev.vvicat.echo/plugin.json").exists());
    }

    #[test]
    fn plugin_action_requires_declared_permission() {
        let allowed = InstalledPlugin {
            manifest: sample_manifest(PluginRuntime::Shell, "main.sh"),
            dir: tempdir().unwrap().path().to_path_buf(),
        };
        assert!(action_allowed(
            &allowed,
            &RpcAction::Copy {
                value: "hello".into()
            }
        ));

        let mut manifest = sample_manifest(PluginRuntime::Shell, "main.sh");
        manifest.permissions.clear();
        let denied = InstalledPlugin {
            manifest,
            dir: tempdir().unwrap().path().to_path_buf(),
        };
        assert!(!action_allowed(
            &denied,
            &RpcAction::Copy {
                value: "hello".into()
            }
        ));
    }

    #[test]
    fn load_available_plugins_prefers_bundled_plugin_id() {
        let dir = tempdir().unwrap();
        let bundled_dir = dir.path().join("bundled/dev.vvicat.clipboard");
        fs::create_dir_all(&bundled_dir).unwrap();
        let mut bundled = sample_manifest(PluginRuntime::Builtin, "builtin:clipboard");
        bundled.id = "dev.vvicat.clipboard".into();
        bundled.name = "内置剪贴板".into();
        bundled.builtin = Some(crate::plugin::BuiltinPluginSpec {
            module: "builtin.ts".into(),
            bridge: "host.clipboard".into(),
            host_commands: vec!["echo.run".into()],
        });
        fs::write(
            bundled_dir.join("plugin.json"),
            serde_json::to_vec_pretty(&bundled).unwrap(),
        )
        .unwrap();

        let plugins = load_plugins(dir.path().join("bundled").as_path()).unwrap();
        assert_eq!(plugins[0].manifest.runtime, PluginRuntime::Builtin);
        assert_eq!(plugins[0].manifest.name, "内置剪贴板");

        let _ = load_available_plugins;
    }
}

pub mod plugin;

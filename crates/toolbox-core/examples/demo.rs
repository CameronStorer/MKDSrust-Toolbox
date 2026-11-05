//! Rebuild the browser demo from generated fixtures only: cargo run -p toolbox-core --example demo.
use toolbox_core::{
    project::{Manifest, Project},
    service::{Request, Service},
    session::RomSession,
    synthetic,
};
fn main() -> toolbox_core::Result<()> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let folder = root.join(format!(
        ".cache/demo-project-{}",
        &toolbox_core::session::hash(&synthetic::rom())[..12]
    ));
    std::fs::create_dir_all(&folder)?;
    let session = RomSession::from_bytes(
        synthetic::rom(),
        std::path::PathBuf::from("/user/roms/synthetic.nds"),
    )?;
    let mut project = if folder.join("project.toml").exists() {
        Project::open(&folder, &session)?
    } else {
        Project::create(&folder,&session,Manifest{id:"studio.colour-study".into(),name:"Colour study".into(),author:"Studio".into(),description:"A small experiment in colour and character. Built entirely from synthetic fixtures.".into(),..Manifest::default()})?
    };
    let path = "data/Studio/demo.carc/art/studio.nclr";
    let mut p = nitro_formats::graphics::Nclr::parse(session.file(path)?)?;
    p.colors[9] = 0x5b1a;
    project.edit(path, p.write(), "Shift palette toward lavender", &session)?;
    let mut service = Service {
        session: Some(session),
        project: Some(project),
        settings: toolbox_core::settings::Settings {
            install_path: "/MKDSrust".into(),
            rom_path: "/user/roms/synthetic.nds".into(),
            ..Default::default()
        },
        persist_settings: false,
        ..Service::default()
    };
    service.dispatch(Request::Select { path: path.into() })?;
    let mut snapshot = service.snapshot()?;
    snapshot.project_path = "/projects/colour-study".into();
    snapshot.rom_path = "/user/roms/synthetic.nds".into();
    let out = root.join("app/src/demo.json");
    std::fs::write(out, serde_json::to_vec_pretty(&snapshot)?)?;
    let wizard =
        RomSession::from_bytes(synthetic::wizard_rom(), "/user/roms/synthetic.nds".into())?;
    std::fs::write(
        root.join("app/public/demo/editing.glb"),
        toolbox_core::scene::CourseScene::load(None, &wizard, "data/Studio/editing.nsbmd")?
            .glb(false)?,
    )?;
    std::fs::write(
        root.join("app/public/demo/editing.png"),
        toolbox_core::preview::png(&synthetic::editing_model().textures.unwrap().decode(0, 0)?)?,
    )?;
    std::fs::write(
        root.join("app/src/demo-wizards.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "messages": wizard.nodes.iter().filter(|n|n.format=="bmg").map(|n|Ok((n.path.clone(),toolbox_core::preview::preview(wizard.file(&n.path)?)?.messages))).collect::<toolbox_core::Result<std::collections::BTreeMap<_,_>>>()?,
            "catalog": toolbox_core::tables::catalog(None, &wizard)?,
            "names": toolbox_core::names::list(None, &wizard)?,
            "game_entries": toolbox_core::game::entries(&wizard.nodes, &toolbox_core::tables::catalog(None, &wizard)?, &toolbox_core::names::list(None, &wizard)?),
            "files": wizard.nodes,
            "related_pages": toolbox_core::related::pages(None, &wizard, &toolbox_core::tables::catalog(None, &wizard)?, &toolbox_core::names::list(None, &wizard)?),
            "scenes": wizard.nodes.iter().filter(|n| n.format=="nsbmd" || n.format=="nsbca").map(|n| Ok((n.path.clone(), toolbox_core::scene::CourseScene::load(None, &wizard, &n.path)?.info))).collect::<toolbox_core::Result<std::collections::BTreeMap<_,_>>>()?,
            "pairings": wizard.pairings,
        }))?,
    )?;
    let session = service.session.as_ref().unwrap();
    let cell = toolbox_core::preview::cell_image(
        None,
        session,
        "data/Studio/demo.carc/art/studio.ncer",
        None,
        None,
        0,
        0,
    )?;
    std::fs::write(
        root.join("app/public/demo/cell.png"),
        toolbox_core::preview::png(&cell)?,
    )?;
    let scene = toolbox_core::scene::CourseScene::load(
        None,
        session,
        "data/Course/studio.carc/course_map.nkm",
    )?;
    std::fs::write(
        root.join("app/src/demo-scene.json"),
        serde_json::to_vec_pretty(&scene.info)?,
    )?;
    std::fs::create_dir_all(root.join("app/public/demo"))?;
    std::fs::write(root.join("app/public/demo/course.glb"), scene.glb(true)?)?;
    std::fs::write(root.join("app/public/demo/model.glb"), scene.glb(false)?)?;
    for (name, path) in [
        (
            "driver",
            "data/KartModelSub.carc/character/mario/P_MR.nsbmd",
        ),
        (
            "static",
            "data/KartModelMainA.carc/character/mario/E_MR.nsbmd",
        ),
    ] {
        let scene = toolbox_core::scene::CourseScene::load(None, &wizard, path)?;
        std::fs::write(
            root.join(format!("app/public/demo/{name}.glb")),
            scene.glb(false)?,
        )?;
    }
    let sound = nitro_formats::audio::Sdat::parse(&synthetic::sound_fixture())?;
    std::fs::write(
        root.join("app/src/demo-audio.json"),
        serde_json::to_vec_pretty(&toolbox_core::audio::resources(&sound)?)?,
    )?;
    std::fs::write(
        root.join("app/public/demo/tone.wav"),
        toolbox_core::audio::wav(&toolbox_core::audio::Pcm {
            rate: 16000,
            channels: vec![sound
                .files
                .get(2)
                .map(|b| {
                    nitro_formats::audio::Swar::parse(b).unwrap().waves[0]
                        .samples()
                        .unwrap()
                })
                .unwrap()],
        })?,
    )?;
    let messages = session
        .nodes
        .iter()
        .filter(|n| n.format == "bmg")
        .map(|n| {
            Ok((
                n.path.clone(),
                toolbox_core::preview::preview(session.file(&n.path)?)?.messages,
            ))
        })
        .collect::<toolbox_core::Result<std::collections::BTreeMap<_, _>>>()?;
    let tiles = nitro_formats::graphics::Ncgr::parse(
        session.file("data/Studio/demo.carc/art/studio.ncgr")?,
    )?;
    let pixels = (0..128 * 128)
        .map(|i| {
            let (x, y) = (i % 128, i / 128);
            tiles.pixels[(y / 8 * 16 + x / 8) * 64 + y % 8 * 8 + x % 8]
        })
        .collect::<Vec<_>>();
    let hex = session
        .nodes
        .iter()
        .map(|n| Ok((n.path.clone(), session.file(&n.path)?.to_vec())))
        .collect::<toolbox_core::Result<std::collections::BTreeMap<_, _>>>()?;
    std::fs::write(
        root.join("app/src/demo-assets.json"),
        serde_json::to_vec(&serde_json::json!({
            "messages": messages, "pixels":pixels, "width":128, "height":128, "hex":hex,
        }))?,
    )?;
    let texture = toolbox_core::preview::image(
        service.project.as_ref(),
        session,
        "data/Studio/texture.nsbtx",
        None,
        None,
        0,
        0,
    )?;
    std::fs::write(
        root.join("app/public/demo/texture.png"),
        toolbox_core::preview::png(&texture)?,
    )?;
    let image = toolbox_core::preview::image(
        service.project.as_ref(),
        service.session.as_ref().unwrap(),
        "data/Studio/demo.carc/art/studio.ncgr",
        Some(path),
        Some("data/Studio/demo.carc/art/studio.nscr"),
        0,
        0,
    )?;
    std::fs::create_dir_all(root.join("app/public/demo"))?;
    std::fs::write(
        root.join("app/public/demo/studio.png"),
        toolbox_core::preview::png(&image)?,
    )?;
    // The application mark is drawn here so no platform icon generator is required.
    let mut rgba = vec![0; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let on = (7..25).contains(&x) && (8..12).contains(&y)
                || (14..18).contains(&x) && (12..25).contains(&y);
            let c = if on {
                [166, 221, 151, 255]
            } else {
                [24, 28, 31, 255]
            };
            let at = (y * 32 + x) * 4;
            rgba[at..at + 4].copy_from_slice(&c);
        }
    }
    let icon = nitro_formats::graphics::Image {
        width: 32,
        height: 32,
        rgba,
    };
    std::fs::write(
        root.join("app/src-tauri/icons/icon.png"),
        toolbox_core::preview::png(&icon)?,
    )?;
    if std::env::args().any(|arg| arg == "--pack") {
        let (bytes, warnings) = toolbox_core::pack::build(
            service.project.as_ref().unwrap(),
            service.session.as_ref().unwrap(),
        )?;
        let output = root.join(".cache/demo-mod.zip");
        std::fs::write(&output, bytes)?;
        println!("{}", output.display());
        for warning in warnings {
            eprintln!("{warning}");
        }
    }
    Ok(())
}

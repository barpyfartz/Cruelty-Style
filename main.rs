use inquire::{CustomType, MultiSelect, Text};
use serde_json::Value;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

// renamed cruelty plus to cruelty style cuz its sounds more tuff
// TODO: FIX THE PEEKOMETER.
// adolf hitler production

fn find_settings_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    
    if let Some(mut data_dir) = dirs::data_dir() {
        data_dir.push("Godot");
        data_dir.push("app_userdata");
        data_dir.push("Cruelty Squad");
        candidates.push(data_dir);
    }
    
    if let Some(mut config_dir) = dirs::config_dir() {
        config_dir.push("Godot");
        config_dir.push("app_userdata");
        config_dir.push("Cruelty Squad");
        candidates.push(config_dir);
    }

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local/share/Steam/steamapps/compatdata/1388770/pfx/drive_c/users/steamuser/AppData/Roaming/Godot/app_userdata/Cruelty Squad"));
        candidates.push(home.join(".local/share/godot/app_userdata/Cruelty Squad"));
        candidates.push(home.join(".wine/drive_c/users/steamuser/AppData/Roaming/Godot/app_userdata/Cruelty Squad"));
        candidates.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/1388770/pfx/drive_c/users/steamuser/AppData/Roaming/Godot/app_userdata/Cruelty Squad"));
    }

    for path in candidates {
        if path.exists() && path.join("settings.save").exists() {
            return Some(path);
        }
    }
    None
}

fn find_game_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.clone());
        candidates.push(cwd.join("Cruelty Squad"));
    }

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".steam/steam/steamapps/common/Cruelty Squad"));
        candidates.push(home.join(".local/share/Steam/steamapps/common/Cruelty Squad"));
        candidates.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/Cruelty Squad"));
    }
    
    candidates.push(PathBuf::from("C:\\Program Files (x86)\\Steam\\steamapps\\common\\Cruelty Squad"));
    candidates.push(PathBuf::from("C:\\Program Files\\Steam\\steamapps\\common\\Cruelty Squad"));
    candidates.push(PathBuf::from("D:\\SteamLibrary\\steamapps\\common\\Cruelty Squad"));

    for path in candidates {
        if path.exists() && path.join("crueltysquad.pck").exists() {
            return Some(path);
        }
    }
    None
}

fn apply_settings(settings_path: &Path, fov: Option<f64>, gamma: Option<f64>) -> io::Result<()> {
    let save_path = settings_path.join("settings.save");
    if !save_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&save_path)?;
    if let Ok(Value::Object(mut map)) = serde_json::from_str::<Value>(&content) {
        if let Some(f) = fov {
            map.insert("FOV".to_string(), serde_json::json!(f));
        }
        if let Some(g) = gamma {
            map.insert("gamma".to_string(), serde_json::json!(g));
        }

        let new_content = serde_json::to_string(&Value::Object(map))?;
        fs::write(&save_path, new_content)?;
    }
    Ok(())
}

fn apply_movement_patch(game_path: &Path) -> io::Result<()> {
    let override_path = game_path.join("override.cfg");
    let mut content = if override_path.exists() {
        fs::read_to_string(&override_path).unwrap_or_default()
    } else {
        String::new()
    };

    if !content.contains("[physics]") {
        content.push_str("\n[physics]\n");
    }
    if !content.contains("common/physics_interpolation") {
        content.push_str("common/physics_interpolation=true\n");
    }
    if !content.contains("common/physics_fps") {
        content.push_str("common/physics_fps=144\n");
    }

    fs::write(&override_path, content)?;
    Ok(())
}

fn patch_pck(game_dir: &Path, fov_limit: f64, apply_parkinson: bool, apply_ceo: bool, apply_dwbt: bool, apply_ishowspeed: bool, apply_ui: bool, num_frames: usize) -> io::Result<()> {
    let original_pck = game_dir.join("crueltysquad.pck");
    let bak_pck = game_dir.join("crueltysquad.pck.bak_plus");
    if !bak_pck.exists() {
        fs::copy(&original_pck, &bak_pck)?;
    }
    let py_script = format!(r#"
import struct
import sys
import hashlib

def patch_pck(src_pck, dst_pck, fov_limit, flags):
    apply_parkinson = flags[0] == "1"
    apply_ceo = flags[1] == "1"
    apply_dwbt = flags[2] == "1"
    apply_ishowspeed = flags[3] == "1"
    apply_ui = flags[4] == "1"
    num_frames = int(flags[5:])
    
    with open(src_pck, 'rb') as f:
        magic = struct.unpack('<I', f.read(4))[0]
        if magic != 0x43504447: return
        pck_ver = struct.unpack('<I', f.read(4))[0]
        eng_major = struct.unpack('<I', f.read(4))[0]
        eng_minor = struct.unpack('<I', f.read(4))[0]
        eng_patch = struct.unpack('<I', f.read(4))[0]
        reserved = f.read(4 * 16)
        fc = struct.unpack('<I', f.read(4))[0]
        files = []
        for i in range(fc):
            pl = struct.unpack('<I', f.read(4))[0]
            raw = f.read(pl)
            path = raw.rstrip(b'\x00').decode('utf-8', 'ignore')
            offset = struct.unpack('<q', f.read(8))[0]
            size = struct.unpack('<q', f.read(8))[0]
            md5 = f.read(16)
            files.append({{'path': path, 'offset': offset, 'size': size, 'raw': raw}})
        
        with open(dst_pck, 'wb') as out:
            out.write(struct.pack('<I', 0x43504447))
            out.write(struct.pack('<I', pck_ver))
            out.write(struct.pack('<I', eng_major))
            out.write(struct.pack('<I', eng_minor))
            out.write(struct.pack('<I', eng_patch))
            out.write(reserved)
            out.write(struct.pack('<I', len(files)))
            index_pos = []
            for fi in files:
                idx_pos = out.tell()
                out.write(struct.pack('<I', len(fi['raw'])))
                out.write(fi['raw'])
                out.write(struct.pack('<q', 0))
                out.write(struct.pack('<q', 0))
                out.write(b'\x00' * 16)
                index_pos.append(idx_pos)
            
            while out.tell() % 64 != 0: out.write(b'\x00')
            
            final_offsets = []
            final_sizes = []
            for fi in files:
                f.seek(fi['offset'])
                data = f.read(fi['size'])
                
                if "Player.gd" in fi['path']:
                    if apply_ui:
                        data = data.replace(b"wrapf(player_view.fov, 4, 120)", f"wrapf(player_view.fov, 4, {{fov_limit}})".encode('utf-8'))
                        data = data.replace(b"wrapf(player_view.fov, 4, 160)", f"wrapf(player_view.fov, 4, {{fov_limit}})".encode('utf-8'))
                    
                    if apply_ceo:
                        ceo_ready = b"\n\tvar __cl=CanvasLayer.new()\n\t__cl.name=\"CEOCanvas\"\n\tvar cr=ColorRect.new()\n\tcr.name=\"TripRect\"\n\tcr.mouse_filter=2\n\tcr.anchor_right=1.0\n\tcr.anchor_bottom=1.0\n\tvar mat=CanvasItemMaterial.new()\n\tmat.blend_mode=1\n\tcr.material=mat\n\t__cl.add_child(cr)\n\tget_tree().get_root().call_deferred(\"add_child\",__cl)\n"
                        data = data.replace(b"func _ready():", b"func _ready():" + ceo_ready)
                        ceo_proc = b"\n\tif not self.has_meta(\"ceo_t\"): self.set_meta(\"ceo_t\", 0.0)\n\tvar ceo_t = self.get_meta(\"ceo_t\") + delta\n\tself.set_meta(\"ceo_t\", ceo_t)\n\tvar __cl_node=get_tree().get_root().get_node_or_null(\"CEOCanvas\")\n\tif __cl_node:\n\t\tvar __tr=__cl_node.get_node_or_null(\"TripRect\")\n\t\tif __tr:\n\t\t\t__tr.color=Color(sin(ceo_t*5.0)*0.5+0.5,sin(ceo_t*6.0)*0.5+0.5,sin(ceo_t*7.0)*0.5+0.5,0.15)\n"
                        data = data.replace(b"func _physics_process(delta):", b"func _physics_process(delta):" + ceo_proc)

                    if apply_ishowspeed:
                        speed_ready = b"\n\tvar cl=CanvasLayer.new()\n\tcl.name=\"SpeedCanvas\"\n\tvar sl=Label.new()\n\tsl.name=\"SpeedLabel\"\n\tsl.add_color_override(\"font_color\", Color(1,1,0))\n\tcl.add_child(sl)\n\tget_tree().get_root().call_deferred(\"add_child\", cl)\n"
                        data = data.replace(b"func _ready():", b"func _ready():" + speed_ready)
                        speed_proc = b"\n\tvar cl=get_tree().get_root().get_node_or_null(\"SpeedCanvas\")\n\tif cl != null:\n\t\tvar sl=cl.get_node_or_null(\"SpeedLabel\")\n\t\tif sl != null:\n\t\t\tsl.text = \"SPEED: \" + str(int(player_velocity.length()))\n\t\t\tvar vp_size = get_viewport().size\n\t\t\tsl.rect_position = Vector2(vp_size.x / 2.0 - sl.rect_size.x / 2.0, vp_size.y - 30)\n"
                        data = data.replace(b"func _physics_process(delta):", b"func _physics_process(delta):" + speed_proc)
                
                if "weapon.gd" in fi['path'] and apply_parkinson:
                    replacement = b"player_weapon.transform.origin.x = lerp(player_weapon.transform.origin.x, -0.035, 5 * delta)\n\t\t\t\tplayer_weapon.transform.origin.y = -0.1\n\t\t\t\tplayer_weapon.transform.origin.z = -0.1"
                    data = data.replace(b"player_weapon.transform.origin.x = lerp(player_weapon.transform.origin.x, - 0.135, 5 * delta)", replacement)

                if "Menu_test.tscn" in fi['path'] and apply_ui:
                    data = data.replace(b'[node name="FOV" type="HSlider" parent="Settings/GridContainer/PanelContainer/VBoxContainer"]', b'[node name="FOV" type="HSlider" parent="Settings/GridContainer/PanelContainer/VBoxContainer"]\nmax_value = 200.0')
                    data = data.replace(b'[node name="GAMMA" type="HSlider" parent="Settings/GridContainer/PanelContainer/VBoxContainer"]', b'[node name="GAMMA" type="HSlider" parent="Settings/GridContainer/PanelContainer/VBoxContainer"]\nmax_value = 3.0')
                
                if "General_Menu.tscn" in fi['path'] and apply_ui:
                    data = data.replace(b'max_value = 160.0', b'max_value = 200.0')

                if "UI.gd" in fi['path'] and apply_dwbt:
                    data = data.replace(b"const HEALTH", b"var HEALTH")
                    data = data.replace(b"const DEATH", b"var DEATH")
                    
                    dwbt_code = f"""
	var cf = []
	for i in range({{num_frames}}):
		var img = Image.new()
		if img.load("user://custom_face_%d.png" % i) == OK:
			img.resize(256, 256, 0)
			var tex = ImageTexture.new()
			tex.create_from_image(img)
			cf.append(tex)
	if cf.size() > 1:
		HEALTH = cf
		DEATH = cf
	elif cf.size() == 1:
		HEALTH = cf[0]
		DEATH = cf[0]
""".encode('utf-8')
                    data = data.replace(b"func _ready():", b"func _ready():" + dwbt_code)
                    
                    anim_proc = b"if typeof(HEALTH) == TYPE_ARRAY:\n\t\thealth_texture.texture = HEALTH[int(OS.get_ticks_msec() / 1000.0 * clamp(100 - health, 24, 100)) % HEALTH.size()]\n\telse:\n\t\thealth_texture.texture = HEALTH\n\t\thealth_texture.texture.fps = clamp(100 - health, 24, 100)"
                    data = data.replace(b"health_texture.texture.fps = clamp(100 - health, 24, 100)", anim_proc)

                off = out.tell()
                final_offsets.append(off)
                final_sizes.append(len(data))
                out.write(data)
                pad = (4 - (len(data) % 4)) % 4
                if pad: out.write(b'\x00' * pad)
            
            for i, fi in enumerate(files):
                out.seek(index_pos[i] + 4 + len(fi['raw']))
                out.write(struct.pack('<q', final_offsets[i]))
                out.write(struct.pack('<q', final_sizes[i]))
                out.write(hashlib.md5(b'').digest())

if __name__ == '__main__':
    patch_pck(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4])
"#);

    let temp_py = std::env::temp_dir().join("cruelty_plus_patcher.py");
    fs::write(&temp_py, py_script)?;

    let tmp_pck = std::env::temp_dir().join("crueltysquad_patched_plus.pck");
    
    let flags = format!("{}{}{}{}{}{}", 
        if apply_parkinson { "1" } else { "0" },
        if apply_ceo { "1" } else { "0" },
        if apply_dwbt { "1" } else { "0" },
        if apply_ishowspeed { "1" } else { "0" },
        if apply_ui { "1" } else { "0" },
        num_frames
    );
    
    #[cfg(target_os = "windows")]
    let python_cmd = "python";
    #[cfg(not(target_os = "windows"))]
    let python_cmd = "python3";

    let status = Command::new(python_cmd)
        .arg(&temp_py)
        .arg(bak_pck.to_str().unwrap())
        .arg(&tmp_pck)
        .arg(fov_limit.to_string())
        .arg(flags)
        .status()?;

    if status.success() {
        fs::copy(&tmp_pck, &original_pck)?;
    }
    Ok(())
}

fn process_image(src_path: &Path, dst_dir: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    let lower_path = src_path.to_string_lossy().to_lowercase();
    let mut num_frames = 1;
    
    if lower_path.ends_with(".gif") {
        use image::AnimationDecoder;
        let file_in = fs::File::open(src_path)?;
        let decoder = image::codecs::gif::GifDecoder::new(file_in)?;
        let frames = decoder.into_frames().collect_frames()?;
        num_frames = frames.len();
        for (i, frame) in frames.into_iter().enumerate() {
            let buffer = frame.into_buffer();
            let resized = image::imageops::resize(&buffer, 196, 196, image::imageops::FilterType::Nearest);
            resized.save(dst_dir.join(format!("custom_face_{}.png", i)))?;
        }
    } else {
        let img = image::open(src_path)?;
        let resized = img.resize_exact(196, 196, image::imageops::FilterType::Nearest);
        resized.save(dst_dir.join("custom_face_0.png"))?;
    }
    
    Ok(num_frames)
}

fn main() {
    let settings_dir = match find_settings_dir() {
        Some(path) => path,
        None => return,
    };

    let game_dir = match find_game_dir() {
        Some(path) => path,
        None => return,
    };

    let options = vec![
        "FOV & Gamma (obv)",
        "MDMA (fix of bug with jittery movement)",
        "ParkinsonSyndrome (shaky hands)", 
        "CEOMindsetTrip (rainbow screen. useless js for fun ig)",
        "HPIcon (custom hp icon - supports PNG, JPG, JPEG, GIF (yes even videogifs))",
        "IShowSpeed (shows ur current speed)",
        "uninstall cruelty style"
    ];

    let selection = MultiSelect::new("cruelty style - select patches:", options)
        .prompt()
        .unwrap_or_default();

    if selection.is_empty() {
        return;
    }

    if selection.iter().any(|s| s.starts_with("uninstall")) {
        let original_pck = game_dir.join("crueltysquad.pck");
        let bak_pck = game_dir.join("crueltysquad.pck.bak_plus");
        if bak_pck.exists() {
            if let Err(e) = fs::copy(&bak_pck, &original_pck) {
                println!("failed to restore backup: {}", e);
            } else {
                fs::remove_file(&bak_pck).unwrap_or_default();
                println!("cruelty style successfully uninstalled. The game is now vanilla");
            }
        } else {
            println!("no backup found. the game is already vanilla or the backup was lost. if its happend after patching reinstall your game");
        }
        return;
    }

    let apply_ui = selection.iter().any(|s| s.starts_with("FOV"));
    let apply_movement = selection.iter().any(|s| s.starts_with("MDMA"));
    let apply_parkinson = selection.iter().any(|s| s.starts_with("ParkinsonSyndrome"));
    let apply_ceo = selection.iter().any(|s| s.starts_with("CEOMindsetTrip"));
    let apply_dwbt = selection.iter().any(|s| s.starts_with("HPIcon"));
    let apply_ishowspeed = selection.iter().any(|s| s.starts_with("IShowSpeed"));

    let mut target_fov = 100.0;
    let mut target_gamma = 1.0;

    if apply_ui {
        target_fov = CustomType::<f64>::new("fov:")
            .with_default(100.0)
            .prompt()
            .unwrap_or(100.0);

        target_gamma = CustomType::<f64>::new("gamma:")
            .with_default(1.0)
            .prompt()
            .unwrap_or(1.0);
    }

    let mut num_frames = 1;

    if apply_dwbt {
        let img_path = Text::new("path to custom face image (.png, .jpg, .jpeg, .gif):")
            .prompt()
            .unwrap_or_default();
        if !img_path.is_empty() {
            let src = Path::new(&img_path);
            if src.exists() {
                if let Ok(frames) = process_image(src, &settings_dir) {
                    num_frames = frames;
                }
            }
        }
    }

    if apply_ui || apply_parkinson || apply_ceo || apply_dwbt || apply_ishowspeed {
        let fov_limit = if apply_ui { 200.0 } else { 120.0 };
        let _ = patch_pck(&game_dir, fov_limit, apply_parkinson, apply_ceo, apply_dwbt, apply_ishowspeed, apply_ui, num_frames);
        
        if apply_ui {
            let _ = apply_settings(&settings_dir, Some(target_fov), Some(target_gamma));
        }
    }

    if apply_movement {
        let _ = apply_movement_patch(&game_dir);
    }
    
    println!("done. be cruel with style.");
}

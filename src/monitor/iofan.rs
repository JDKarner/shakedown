use std::fs;
use std::path::PathBuf;
use std::io::Read;
use std::sync::{Arc, Mutex};

/// Monitor that periodically updates a shared fan list
pub struct FanMonitor {
    fans: Arc<Mutex<Vec<Fan>>>,
}

impl FanMonitor {
    pub fn new(fans: Arc<Mutex<Vec<Fan>>>) -> Self {
        Self { fans }
    }

    /// Refresh the shared fan list from the system (system76 or sysfs)
    pub fn update(&mut self) {
        let readings = read_fans();
        if let Ok(mut f) = self.fans.lock() {
            *f = readings;
        }
    }

    /// Convenience accessor returning a snapshot of current readings
    pub fn get_readings(&self) -> Vec<Fan> {
        if let Ok(f) = self.fans.lock() {
            f.clone()
        } else {
            Vec::new()
        }
    }
}

/// Lightweight representation of a fan reading
#[derive(Debug, Clone)]
pub struct Fan {
    /// unique id or sysfs path
    pub id: String,
    /// human label if available
    pub label: Option<String>,
    /// RPM reading if available
    pub rpm: Option<u32>,
    /// duty / PWM percentage if available (0..=100)
    pub duty: Option<u8>,
    /// associated temperature if available (°C)
    pub temp: Option<f32>,
}

impl Fan {
    pub fn new(id: String) -> Self {
        Fan { id, label: None, rpm: None, duty: None, temp: None }
    }
}

/// Try to read fan data using system76-power if compiled with the `system76` feature.
/// Falls back to reading `/sys/class/hwmon` otherwise.
pub fn read_fans() -> Vec<Fan> {
    // Prefer system76 integration when the feature is enabled
    #[cfg(feature = "system76")]
    {
        if let Ok(fans) = read_system76_fans() {
            if !fans.is_empty() {
                return fans;
            }
        }
    }

    // If lm-sensors is enabled, try it next (prefer chips like system76_io)
    #[cfg(feature = "lmsensors")]
    {
        if let Ok(fans) = read_lmsensors_fans() {
            if !fans.is_empty() {
                return fans;
            }
        }
    }

    // Fallback to sysfs
    read_sysfs_fans()
}

#[cfg(feature = "system76")]
fn read_system76_fans() -> Result<Vec<Fan>, String> {
    use system76_power::fan::FanDaemon;

    // Detect if `nvidia-smi` exists; FanDaemon accepts a boolean flag to enable NV support
    let nvidia_exists = std::process::Command::new("which").arg("nvidia-smi").stdout(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false);

    let daemon = FanDaemon::new(nvidia_exists);

    // Get highest temp (in thousandths of a degree) and computed duty (0..255)
    if let Some(temp_thousandths) = daemon.get_temp() {
        let duty_opt = daemon.get_duty(temp_thousandths);

        let mut fan = Fan::new("system76-power".to_string());
        // convert temp to °C
        fan.temp = Some((temp_thousandths as f32) / 1000.0);
        // convert duty from 0..255 PWM to percentage 0..=100
        fan.duty = duty_opt.map(|d| ((d as u32 * 100) / 255) as u8);

        return Ok(vec![fan]);
    }

    Ok(Vec::new())
}

// Sysfs fallback implementation
fn read_sysfs_fans() -> Vec<Fan> {
    let mut fans = Vec::new();

    // Look for hwmon directories
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }

            // Read name if present
            let name = fs::read_to_string(path.join("name")).ok().map(|s| s.trim().to_string());

            // For each fanN_input file in this hwmon
            if let Ok(files) = fs::read_dir(&path) {
                for f in files.flatten() {
                    let fname = f.file_name().to_string_lossy().into_owned();
                    if fname.starts_with("fan") && fname.ends_with("_input") {
                        let mut fan = Fan::new(format!("{}/{}", path.display(), fname));
                        fan.label = name.clone();

                        // Try to read rpm
                        let _ = read_value(path.join(&fname)).and_then(|s| s.parse::<u32>().ok()).map(|v| fan.rpm = Some(v));

                        // Try to read corresponding PWM (pwmN) or duty (pwmN or fanN_target) as percent
                        let pwm_name = fname.replacen("_input", "_pwm", 1);
                        if let Some(pwm) = read_value(path.join(&pwm_name)).and_then(|s| s.parse::<u32>().ok()) {
                            // pwm is typically 0..=255 or 0..=100; normalize if >100
                            let duty = if pwm > 100 { ((pwm as f32) / 255.0 * 100.0).round() as u8 } else { pwm as u8 };
                            fan.duty = Some(duty);
                        }

                        // Try to find an associated temp input (tempN_input) in same hwmon
                        // Heuristic: search for any temp*_input and use the first available
                        if fan.temp.is_none() {
                            if let Ok(temps) = fs::read_dir(&path) {
                                for t in temps.flatten() {
                                    let tn = t.file_name().to_string_lossy().into_owned();
                                    if tn.starts_with("temp") && tn.ends_with("_input") {
                                        if let Some(tval) = read_value(t.path()).and_then(|s| s.parse::<f32>().ok()) {
                                            // temp inputs are often in millidegrees (e.g. 42000)
                                            fan.temp = if tval > 1000.0 { Some(tval / 1000.0) } else { Some(tval) };
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        fans.push(fan);
                    }
                }
            }
        }
    }

    fans
}

fn read_value(path: PathBuf) -> Option<String> {
    let mut s = String::new();
    if let Ok(mut f) = fs::File::open(path) {
        if f.read_to_string(&mut s).is_ok() {
            return Some(s.trim().to_string());
        }
    }
    None
}

// Optional lm-sensors backend. When the `lmsensors` feature is enabled we can
// call the `sensors -j` command and parse the JSON output to extract fan/pwm
// and temperature readings from chips such as `system76_io`.
#[cfg(feature = "lmsensors")]
fn read_lmsensors_fans() -> Result<Vec<Fan>, String> {
    use serde_json::Value;
    use std::process::Command;
    use std::collections::HashMap;

    let out = Command::new("sensors").arg("-j").output().map_err(|e| format!("failed to run sensors: {}", e))?;
    if !out.status.success() {
        return Err(format!("sensors returned non-zero exit: {}", out.status));
    }

    let v: Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("failed to parse sensors JSON: {}", e))?;

    // We'll build a map of fan_label -> Fan so we can attach rpm/pwm/temp to the
    // correct labeled fan (e.g. "CPU fan" and "CPU temp" get associated).
    let mut fans_map: HashMap<String, Fan> = HashMap::new();
    // map of map_key -> chip name so we can merge entries belonging to the same chip
    let mut map_key_to_chip: HashMap<String, String> = HashMap::new();
    // Collect orphan temps to associate after we know fan labels
    let mut temps: Vec<(String, f32)> = Vec::new();

    if let Some(obj) = v.as_object() {
        for (chip_name, chip_val) in obj {
            if let Some(chip_obj) = chip_val.as_object() {
                for (sensor_key, sensor_val) in chip_obj {
                    let key = sensor_key.clone();
                    let key_lower = key.to_lowercase();

                    // Determine sensor label if present. If the inner object doesn't provide
                    // a `label`, some sensors use the key name itself as the human label
                    // (e.g. "CPU fan"). Use that unless the key is a generic one like
                    // "Adapter".
                    let mut label_opt: Option<String> = None;
                    if let Some(sobj) = sensor_val.as_object() {
                        if let Some(lbl) = sobj.get("label").and_then(|l| l.as_str()) {
                            label_opt = Some(lbl.to_string());
                        }
                    }

                    // If no explicit label, and the key looks like a readable label, use it
                    if label_opt.is_none() {
                        let kl = key_lower.trim();
                        if kl != "adapter" && (kl.contains("fan") || kl.contains("temp") || kl.contains("pwm") || kl.contains("cpu") || kl.contains("gpu")) {
                            label_opt = Some(key.clone());
                        }
                    }

                    // Helper to pick a map key (prefer label, fallback to chip:key)
                    let map_key = label_opt.clone().unwrap_or_else(|| format!("{}:{}", chip_name, key));
                    // remember which chip this map_key came from so we can relate PWM to fans
                    map_key_to_chip.insert(map_key.clone(), chip_name.clone());

                    // We'll create or get fan entries inline below (avoid closures returning references).

                    // If sensor_val is object, try to read its `input` numeric or any nested numeric fields
                    if let Some(sobj) = sensor_val.as_object() {
                        // First try common keys `input` / `value` for simplicity
                        if let Some(input) = sobj.get("input").or_else(|| sobj.get("value")) {
                            if let Some(num) = input.as_f64() {
                                // fan RPMs: keys like fan1 or labels that contain "fan"
                                if key_lower.contains("fan") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("fan")) {
                                    let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                        let mut f = Fan::new(map_key.clone());
                                        f.label = label_opt.clone();
                                        f
                                    });
                                    if entry.label.is_none() { entry.label = label_opt.clone(); }
                                    entry.rpm = Some(num as u32);
                                }

                                // pwm keys (pwm1) -> duty or raw pwm
                                else if key_lower.contains("pwm") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("pwm")) {
                                    let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                        let mut f = Fan::new(map_key.clone());
                                        f.label = label_opt.clone();
                                        f
                                    });
                                    if entry.label.is_none() { entry.label = label_opt.clone(); }
                                    let pwm = num as f32;
                                    let duty = if pwm > 100.0 { ((pwm / 255.0) * 100.0).round() as u8 } else { pwm.round() as u8 };
                                    entry.duty = Some(duty);
                                }

                                // temp keys -> associate later to nearest fan by base label
                                else if key_lower.contains("temp") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("temp")) {
                                    let t = if num > 1000.0 { (num / 1000.0) as f32 } else { num as f32 };
                                    temps.push((map_key.clone(), t));
                                }
                            }
                        } else {
                            // Some chips put the actual readings under nested keys like `fan1_input`, `temp1_input`, `pwm1`, etc.
                            for (inner_k, inner_v) in sobj {
                                if let Some(num) = inner_v.as_f64() {
                                    let ik = inner_k.to_lowercase();

                                    // fan RPMs
                                    if ik.contains("fan") || key_lower.contains("fan") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("fan")) {
                                        let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                            let mut f = Fan::new(map_key.clone());
                                            f.label = label_opt.clone();
                                            f
                                        });
                                        if entry.label.is_none() { entry.label = label_opt.clone(); }
                                        entry.rpm = Some(num as u32);
                                    }
                                    // pwm entries
                                    else if ik.contains("pwm") || key_lower.contains("pwm") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("pwm")) {
                                        let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                            let mut f = Fan::new(map_key.clone());
                                            f.label = label_opt.clone();
                                            f
                                        });
                                        if entry.label.is_none() { entry.label = label_opt.clone(); }
                                        let pwm = num as f32;
                                        let duty = if pwm > 100.0 { ((pwm / 255.0) * 100.0).round() as u8 } else { pwm.round() as u8 };
                                        entry.duty = Some(duty);
                                    }
                                    // temps
                                    else if ik.contains("temp") || key_lower.contains("temp") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("temp")) {
                                        let t = if num > 1000.0 { (num / 1000.0) as f32 } else { num as f32 };
                                        temps.push((map_key.clone(), t));
                                    }
                                }
                            }
                        }
                    }

                    // If sensor_val is numeric directly, handle similarly
                    else if let Some(num) = sensor_val.as_f64() {
                        if key_lower.contains("fan") {
                            let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                let mut f = Fan::new(map_key.clone());
                                f.label = label_opt.clone();
                                f
                            });
                            if entry.label.is_none() { entry.label = label_opt.clone(); }
                            entry.rpm = Some(num as u32);
                        } else if key_lower.contains("pwm") {
                            let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                let mut f = Fan::new(map_key.clone());
                                f.label = label_opt.clone();
                                f
                            });
                            if entry.label.is_none() { entry.label = label_opt.clone(); }
                            let pwm = num as f32;
                            let duty = if pwm > 100.0 { ((pwm / 255.0) * 100.0).round() as u8 } else { pwm.round() as u8 };
                            entry.duty = Some(duty);
                        } else if key_lower.contains("temp") {
                            let t = if num > 1000.0 { (num / 1000.0) as f32 } else { num as f32 };
                            temps.push((map_key.clone(), t));
                        }
                    }
                }
            }
        }
    }

    // Associate temps with fans heuristically by matching base labels
    for (temp_key, tval) in temps {
        // compute base token (remove words like fan/temp/pwm/adapter)
        let base = temp_key.to_lowercase()
            .replace("fan", "")
            .replace("temp", "")
            .replace("pwm", "")
            .replace("adapter", "")
            .replace(':', " ")
            .trim()
            .to_string();

        // Try to find an existing fan whose key or label contains base
        let mut assigned = false;
        if !base.is_empty() {
            for (_k, f) in fans_map.iter_mut() {
                if f.label.as_ref().map_or(false, |lbl| lbl.to_lowercase().contains(&base)) || f.id.to_lowercase().contains(&base) {
                    f.temp = Some(tval);
                    assigned = true;
                    break;
                }
            }
        }

        // Fallback: if not assigned, try to attach to any fan in same chip name prefix
        if !assigned {
            // If temp_key contained chip:subkey format, try to match chip prefix
            if let Some(pos) = temp_key.find(":") {
                let chip = &temp_key[..pos];
                for (_k, f) in fans_map.iter_mut() {
                    if f.id.starts_with(chip) {
                        f.temp = Some(tval);
                        assigned = true;
                        break;
                    }
                }
            }
        }

        // If still not assigned, create a new fan entry for this temperature
        if !assigned {
            let mut f = Fan::new(temp_key.clone());
            f.temp = Some(tval);
            fans_map.insert(temp_key, f);
        }
    }

    // Merge duplicate labels (e.g., multiple keys referring to "CPU fan")
    // Merge duplicate labels using a snapshot to avoid borrowing conflicts
    let mut to_remove: Vec<String> = Vec::new();
    let mut label_index: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for key in fans_map.keys().cloned().collect::<Vec<_>>() {
        if let Some(fan) = fans_map.get(&key).cloned() {
            let norm = fan.label.as_deref().unwrap_or(&fan.id).to_lowercase();
            if let Some(existing_key) = label_index.get(&norm) {
                if let Some(existing) = fans_map.get_mut(existing_key) {
                    if existing.rpm.is_none() { existing.rpm = fan.rpm; }
                    if existing.duty.is_none() { existing.duty = fan.duty; }
                    if existing.temp.is_none() { existing.temp = fan.temp; }
                }
                to_remove.push(key);
            } else {
                label_index.insert(norm, key.clone());
            }
        }
    }
    for k in to_remove { fans_map.remove(&k); }

    // Merge pwm-only entries (pwmN / "pwm1") into the appropriate fan when possible
    let mut remove_pwm: Vec<String> = Vec::new();
    // Snapshot of keys+meta so we can search without borrowing the map
    let snapshot: Vec<(String, Option<String>, Option<u32>, Option<String>)> = fans_map.iter().map(|(k,v)| (k.clone(), v.label.clone(), v.rpm, map_key_to_chip.get(k).cloned())).collect();
    for (key, _lbl, _rpm, _chip) in snapshot.iter() {
        if let Some(fan) = fans_map.get(key).cloned() {
            if fan.duty.is_some() && fan.rpm.is_none() {
                // Try chip association (we saved a map_key->chip earlier)
                let chip_prefix = map_key_to_chip.get(key).cloned().unwrap_or_default();

                if !chip_prefix.is_empty() {
                    if let Some((tkey, _, _, Some(chip2))) = snapshot.iter().find(|(k2, _lbl2, rpm2, chip2)| k2.as_str() != key.as_str() && (rpm2.is_some() || _lbl2.as_ref().map(|l| l.to_lowercase().contains("cpu")).unwrap_or(false)) && chip2.as_ref().map(|c| c == &chip_prefix).unwrap_or(false)) {
                        if let Some(t) = fans_map.get_mut(tkey) {
                            if t.duty.is_none() { t.duty = fan.duty; }
                        }
                        remove_pwm.push(key.clone());
                        continue;
                    }
                }

                // Fallback: match by base token from label/id (e.g., "cpu" in "pwm1")
                let base = fan.label.as_deref().unwrap_or(&fan.id).to_lowercase().replace("pwm", "").replace(|c: char| c.is_ascii_digit(), "").trim().to_string();
                if !base.is_empty() {
                    if let Some((tkey, lbl2, _, _)) = snapshot.iter().find(|(k2, lbl2, _, _)| k2.as_str() != key.as_str() && (lbl2.as_ref().map(|l| l.to_lowercase().contains(&base)).unwrap_or(false) || k2.to_lowercase().contains(&base))) {
                        if let Some(t) = fans_map.get_mut(tkey) {
                            if t.duty.is_none() { t.duty = fan.duty; }
                        }
                        remove_pwm.push(key.clone());
                        continue;
                    }
                }
            }
        }
    }
    for k in remove_pwm { fans_map.remove(&k); }

    // Remove temp-only entries (no rpm and no duty)
    let temp_only: Vec<String> = fans_map.iter().filter_map(|(k, v)| if v.rpm.is_none() && v.duty.is_none() { Some(k.clone()) } else { None }).collect();
    for k in temp_only { fans_map.remove(&k); }

    // Return a stable, sorted list so GUI entries don't shuffle on updates
    let mut fans: Vec<Fan> = fans_map.into_values().collect();
    fans.sort_by_key(|f| f.label.clone().unwrap_or_else(|| f.id.clone()).to_lowercase());
    Ok(fans)
}

// Test helper: parse sensors JSON directly (useful for unit tests)
#[cfg(feature = "lmsensors")]
pub fn parse_lmsensors_json(input: &str) -> Result<Vec<Fan>, String> {
    let v: serde_json::Value = serde_json::from_str(input).map_err(|e| format!("failed to parse JSON: {}", e))?;

    // Reuse the same logic as the live parser by re-invoking read_lmsensors_fans's parsing
    // block but operating on the provided JSON value.
    // We'll duplicate the essential parsing steps here so tests don't need to invoke `sensors`.
    use std::collections::HashMap;

    let mut fans_map: HashMap<String, Fan> = HashMap::new();
    let mut map_key_to_chip: HashMap<String, String> = HashMap::new();
    let mut temps: Vec<(String, f32)> = Vec::new();

    if let Some(obj) = v.as_object() {
        for (chip_name, chip_val) in obj {
            if let Some(chip_obj) = chip_val.as_object() {
                for (sensor_key, sensor_val) in chip_obj {
                    let key = sensor_key.clone();
                    let key_lower = key.to_lowercase();

                    let mut label_opt: Option<String> = None;
                    if let Some(sobj) = sensor_val.as_object() {
                        if let Some(lbl) = sobj.get("label").and_then(|l| l.as_str()) {
                            label_opt = Some(lbl.to_string());
                        }
                    }

                    if label_opt.is_none() {
                        let kl = key_lower.trim();
                        if kl != "adapter" && (kl.contains("fan") || kl.contains("temp") || kl.contains("pwm") || kl.contains("cpu") || kl.contains("gpu")) {
                            label_opt = Some(key.clone());
                        }
                    }

                    let map_key = label_opt.clone().unwrap_or_else(|| format!("{}:{}", chip_name, key));
                    map_key_to_chip.insert(map_key.clone(), chip_name.clone());

                    if let Some(sobj) = sensor_val.as_object() {
                        if let Some(input) = sobj.get("input").or_else(|| sobj.get("value")) {
                            if let Some(num) = input.as_f64() {
                                if key_lower.contains("fan") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("fan")) {
                                    let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                        let mut f = Fan::new(map_key.clone());
                                        f.label = label_opt.clone();
                                        f
                                    });
                                    if entry.label.is_none() { entry.label = label_opt.clone(); }
                                    entry.rpm = Some(num as u32);
                                } else if key_lower.contains("pwm") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("pwm")) {
                                    let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                        let mut f = Fan::new(map_key.clone());
                                        f.label = label_opt.clone();
                                        f
                                    });
                                    if entry.label.is_none() { entry.label = label_opt.clone(); }
                                    let pwm = num as f32;
                                    let duty = if pwm > 100.0 { ((pwm / 255.0) * 100.0).round() as u8 } else { pwm.round() as u8 };
                                    entry.duty = Some(duty);
                                } else if key_lower.contains("temp") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("temp")) {
                                    let t = if num > 1000.0 { (num / 1000.0) as f32 } else { num as f32 };
                                    temps.push((map_key.clone(), t));
                                }
                            }
                        } else {
                            for (inner_k, inner_v) in sobj {
                                if let Some(num) = inner_v.as_f64() {
                                    let ik = inner_k.to_lowercase();
                                    if ik.contains("fan") || key_lower.contains("fan") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("fan")) {
                                        let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                            let mut f = Fan::new(map_key.clone());
                                            f.label = label_opt.clone();
                                            f
                                        });
                                        if entry.label.is_none() { entry.label = label_opt.clone(); }
                                        entry.rpm = Some(num as u32);
                                    } else if ik.contains("pwm") || key_lower.contains("pwm") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("pwm")) {
                                        let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                            let mut f = Fan::new(map_key.clone());
                                            f.label = label_opt.clone();
                                            f
                                        });
                                        if entry.label.is_none() { entry.label = label_opt.clone(); }
                                        let pwm = num as f32;
                                        let duty = if pwm > 100.0 { ((pwm / 255.0) * 100.0).round() as u8 } else { pwm.round() as u8 };
                                        entry.duty = Some(duty);
                                    } else if ik.contains("temp") || key_lower.contains("temp") || label_opt.as_ref().map_or(false, |l| l.to_lowercase().contains("temp")) {
                                        let t = if num > 1000.0 { (num / 1000.0) as f32 } else { num as f32 };
                                        temps.push((map_key.clone(), t));
                                    }
                                }
                            }
                        }
                    } else if let Some(num) = sensor_val.as_f64() {
                        if key_lower.contains("fan") {
                            let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                let mut f = Fan::new(map_key.clone());
                                f.label = label_opt.clone();
                                f
                            });
                            if entry.label.is_none() { entry.label = label_opt.clone(); }
                            entry.rpm = Some(num as u32);
                        } else if key_lower.contains("pwm") {
                            let entry = fans_map.entry(map_key.clone()).or_insert_with(|| {
                                let mut f = Fan::new(map_key.clone());
                                f.label = label_opt.clone();
                                f
                            });
                            if entry.label.is_none() { entry.label = label_opt.clone(); }
                            let pwm = num as f32;
                            let duty = if pwm > 100.0 { ((pwm / 255.0) * 100.0).round() as u8 } else { pwm.round() as u8 };
                            entry.duty = Some(duty);
                        } else if key_lower.contains("temp") {
                            let t = if num > 1000.0 { (num / 1000.0) as f32 } else { num as f32 };
                            temps.push((map_key.clone(), t));
                        }
                    }
                }
            }
        }
    }

    // Associate temps with fans heuristically by matching base labels
    for (temp_key, tval) in temps {
        // compute base token (remove words like fan/temp/pwm/adapter)
        let base = temp_key.to_lowercase()
            .replace("fan", "")
            .replace("temp", "")
            .replace("pwm", "")
            .replace("adapter", "")
            .replace(':', " ")
            .trim()
            .to_string();

        // Try to find an existing fan whose key or label contains base
        let mut assigned = false;
        if !base.is_empty() {
            for (_k, f) in fans_map.iter_mut() {
                if f.label.as_ref().map_or(false, |lbl| lbl.to_lowercase().contains(&base)) || f.id.to_lowercase().contains(&base) {
                    f.temp = Some(tval);
                    assigned = true;
                    break;
                }
            }
        }

        // Fallback: if not assigned, try to attach to any fan in same chip name prefix
        if !assigned {
            // If temp_key contained chip:subkey format, try to match chip prefix
            if let Some(pos) = temp_key.find(":") {
                let chip = &temp_key[..pos];
                for (_k, f) in fans_map.iter_mut() {
                    if f.id.starts_with(chip) {
                        f.temp = Some(tval);
                        assigned = true;
                        break;
                    }
                }
            }
        }

        // If still not assigned, create a new fan entry for this temperature
        if !assigned {
            let mut f = Fan::new(temp_key.clone());
            f.temp = Some(tval);
            fans_map.insert(temp_key, f);
        }
    }

    // Merge duplicate labels (e.g., multiple keys referring to "CPU fan")
    // Merge duplicate labels using a snapshot to avoid borrowing conflicts
    let mut to_remove: Vec<String> = Vec::new();
    let mut label_index: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for key in fans_map.keys().cloned().collect::<Vec<_>>() {
        if let Some(fan) = fans_map.get(&key).cloned() {
            let norm = fan.label.as_deref().unwrap_or(&fan.id).to_lowercase();
            if let Some(existing_key) = label_index.get(&norm) {
                if let Some(existing) = fans_map.get_mut(existing_key) {
                    if existing.rpm.is_none() { existing.rpm = fan.rpm; }
                    if existing.duty.is_none() { existing.duty = fan.duty; }
                    if existing.temp.is_none() { existing.temp = fan.temp; }
                }
                to_remove.push(key);
            } else {
                label_index.insert(norm, key.clone());
            }
        }
    }
    for k in to_remove { fans_map.remove(&k); }

    // Merge pwm-only entries (pwmN / "pwm1") into the appropriate fan when possible
    let mut remove_pwm: Vec<String> = Vec::new();
    // Snapshot of keys+meta so we can search without borrowing the map
    let snapshot: Vec<(String, Option<String>, Option<u32>, Option<String>)> = fans_map.iter().map(|(k,v)| (k.clone(), v.label.clone(), v.rpm, map_key_to_chip.get(k).cloned())).collect();
    for (key, _lbl, _rpm, _chip) in snapshot.iter() {
        if let Some(fan) = fans_map.get(key).cloned() {
            if fan.duty.is_some() && fan.rpm.is_none() {
                // Try chip association (we saved a map_key->chip earlier)
                let chip_prefix = map_key_to_chip.get(key).cloned().unwrap_or_default();

                if !chip_prefix.is_empty() {
                    if let Some((tkey, _, _, Some(chip2))) = snapshot.iter().find(|(k2, _lbl2, rpm2, chip2)| k2.as_str() != key.as_str() && (rpm2.is_some() || _lbl2.as_ref().map(|l| l.to_lowercase().contains("cpu")).unwrap_or(false)) && chip2.as_ref().map(|c| c == &chip_prefix).unwrap_or(false)) {
                        if let Some(t) = fans_map.get_mut(tkey) {
                            if t.duty.is_none() { t.duty = fan.duty; }
                        }
                        remove_pwm.push(key.clone());
                        continue;
                    }
                }

                // Fallback: match by base token from label/id (e.g., "cpu" in "pwm1")
                let base = fan.label.as_deref().unwrap_or(&fan.id).to_lowercase().replace("pwm", "").replace(|c: char| c.is_ascii_digit(), "").trim().to_string();
                if !base.is_empty() {
                    if let Some((tkey, lbl2, _, _)) = snapshot.iter().find(|(k2, lbl2, _, _)| k2.as_str() != key.as_str() && (lbl2.as_ref().map(|l| l.to_lowercase().contains(&base)).unwrap_or(false) || k2.to_lowercase().contains(&base))) {
                        if let Some(t) = fans_map.get_mut(tkey) {
                            if t.duty.is_none() { t.duty = fan.duty; }
                        }
                        remove_pwm.push(key.clone());
                        continue;
                    }
                }
            }
        }
    }
    for k in remove_pwm { fans_map.remove(&k); }

    // Remove temp-only entries (no rpm and no duty)
    let temp_only: Vec<String> = fans_map.iter().filter_map(|(k, v)| if v.rpm.is_none() && v.duty.is_none() { Some(k.clone()) } else { None }).collect();
    for k in temp_only { fans_map.remove(&k); }

    // Return a stable, sorted list so GUI entries don't shuffle on updates
    let mut fans: Vec<Fan> = fans_map.into_values().collect();
    fans.sort_by_key(|f| f.label.clone().unwrap_or_else(|| f.id.clone()).to_lowercase());
    Ok(fans)
}

#[cfg(all(test, feature = "lmsensors"))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::env;

    #[test]
    fn test_parse_cpu_fan_merges_pwm() {
        // Create a temporary dir to hold our fake `sensors` executable
        let tmpdir = tempfile::tempdir().expect("tempdir");
        let script_path = tmpdir.path().join("sensors");

        // JSON fixture includes an ACPI chip with CPU fan + pwm1 and a coretemp chip
        let fixture = r#"
{
  "acpi-virtual-0": {
    "Adapter": "ACPI interface",
    "CPU fan": { "fan1_input": 0.0 },
    "CPU temp": { "temp1_input": 43.0 },
    "pwm1": { "pwm1": 0.0 }
  },
  "coretemp-isa-0000": {
    "Adapter": "ISA adapter",
    "Core 0": { "temp1_input": 100.0 }
  }
}
"#;

        // Write a small shell script that prints the fixture when called with -j
        let script = format!("#!/bin/sh\nif [ \"$1\" = \"-j\" ]; then\ncat <<'JSON'\n{}\nJSON\nexit 0\nfi\necho \"Unsupported args\" >&2\nexit 1\n", fixture);
        fs::write(&script_path, script).expect("write script");
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();

        // Prepend temp dir to PATH
        let orig_path = env::var_os("PATH").unwrap_or_default();
        let mut new_path = tmpdir.path().to_owned();
        new_path.push("");
        let new_path_str = format!("{}:{}", tmpdir.path().display(), orig_path.to_string_lossy());
        env::set_var("PATH", &new_path_str);

        // Run the real function which will invoke our fake `sensors -j`
        let fans = read_lmsensors_fans().expect("read_lmsensors_fans");

        // Restore PATH
        env::set_var("PATH", orig_path);

        // Find CPU fan
        let cpu = fans.iter().find(|f| f.label.as_deref() == Some("CPU fan") || f.id.contains("CPU fan"));
        assert!(cpu.is_some(), "CPU fan should be present");
        let cpu = cpu.unwrap();
        assert_eq!(cpu.rpm, Some(0));
        assert_eq!(cpu.duty, Some(0));
        assert_eq!(cpu.temp, Some(43.0));

        // Ensure coretemp doesn't appear as a fan
        assert!(fans.iter().all(|f| !f.id.contains("coretemp") && f.label.as_deref() != Some("Core 0")));

        // Ensure pwm1 does not exist as a separate entry
        assert!(fans.iter().all(|f| f.label.as_deref() != Some("pwm1") && !f.id.contains("pwm1")));
    }
}


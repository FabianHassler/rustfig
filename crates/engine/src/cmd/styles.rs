//! Style commands (E6): families of named attribute sets kept in the
//! preferences, after xfig's style manager (`w_style.c`: families of styles;
//! Add/Delete family, Add/Delete/Set style). Apply sets the indicator only.

use super::{always, opt_bool, string, CmdResult, Command};
use crate::styles::{clean_name, AttrStyle, NamedStyle, StyleFamily, MAX_FAMILIES, MAX_STYLES};
use crate::{Engine, EngineError};
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "style.family_add",
            label: "Add style family",
            menu: "",
            shortcut: "",
            params: "{\"name\": text (1-64 characters)}  — add an empty family to the settings file (at most 32 families; xfig: 16). Returns {count}",
            mutates: false,
            enabled: always,
            run: family_add,
        },
        Command {
            id: "style.family_delete",
            label: "Delete style family",
            menu: "",
            shortcut: "",
            params: "{\"family\": text}  — remove the family and its styles",
            mutates: false,
            enabled: always,
            run: family_delete,
        },
        Command {
            id: "style.family_rename",
            label: "Rename style family",
            menu: "",
            shortcut: "",
            params: "{\"family\": text, \"new_name\": text}  — rename a family; a family of the new name is an error",
            mutates: false,
            enabled: always,
            run: family_rename,
        },
        Command {
            id: "style.save",
            label: "Save style",
            menu: "",
            shortcut: "",
            params: "{\"family\": text, \"name\": text (1-64 characters)}  — keep the indicator's attributes (depth, line width/style/join/cap/dash/dot gap, pen and fill colour/fill, box radius, arrows, text font/size/flags/justification, arc type) under the name in the family, in the settings file (not in the drawing); an existing style of that name is replaced; at most 64 styles per family (xfig: 16). Returns {count, replaced}",
            mutates: false,
            enabled: always,
            run: save,
        },
        Command {
            id: "style.apply",
            label: "Apply style",
            menu: "",
            shortcut: "",
            params: "{\"family\": text, \"name\": text}  — set the indicator's attributes to the style's (new objects get them; as xfig's Set); drawn objects are untouched",
            mutates: false,
            enabled: always,
            run: apply,
        },
        Command {
            id: "style.delete",
            label: "Delete style",
            menu: "",
            shortcut: "",
            params: "{\"family\": text, \"name\": text}  — remove the style from its family",
            mutates: false,
            enabled: always,
            run: delete,
        },
        Command {
            id: "style.rename",
            label: "Rename style",
            menu: "",
            shortcut: "",
            params: "{\"family\": text, \"name\": text, \"new_name\": text, \"overwrite\"?: bool}  — rename a style; a style of the new name in the family is an error unless overwrite is true",
            mutates: false,
            enabled: always,
            run: rename,
        },
        Command {
            id: "style.list",
            label: "List styles",
            menu: "",
            shortcut: "",
            params: "{\"family\"?: text, \"name\"?: text}  — without a family {families: [{name, count}]}; with one {family, styles: [{name}]}; with a name too, that style's attributes are included as attrs",
            mutates: false,
            enabled: always,
            run: list,
        },
    ]
}

fn name_of(p: &Value, key: &str) -> Result<String, EngineError> {
    clean_name(&string(p, key)?).map_err(EngineError::bad_params)
}

fn family_mut<'a>(e: &'a mut Engine, name: &str) -> Result<&'a mut StyleFamily, EngineError> {
    e.prefs
        .rustfig
        .style_families
        .iter_mut()
        .find(|f| f.name == name)
        .ok_or_else(|| EngineError::bad_params(format!("no style family {name:?}")))
}

fn family_ref<'a>(e: &'a Engine, name: &str) -> Result<&'a StyleFamily, EngineError> {
    e.prefs
        .rustfig
        .style_families
        .iter()
        .find(|f| f.name == name)
        .ok_or_else(|| EngineError::bad_params(format!("no style family {name:?}")))
}

fn find<'a>(e: &'a Engine, family: &str, name: &str) -> Result<&'a NamedStyle, EngineError> {
    family_ref(e, family)?
        .styles
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| EngineError::bad_params(format!("no style {name:?} in family {family:?}")))
}

fn family_add(e: &mut Engine, p: &Value) -> CmdResult {
    let name = name_of(p, "name")?;
    let fams = &mut e.prefs.rustfig.style_families;
    if fams.iter().any(|f| f.name == name) {
        return Err(EngineError::bad_params(format!("a family named {name:?} exists")));
    }
    if fams.len() >= MAX_FAMILIES {
        return Err(EngineError::bad_params(format!("at most {MAX_FAMILIES} families")));
    }
    fams.push(StyleFamily {
        name,
        styles: Vec::new(),
    });
    e.prefs_dirty = true;
    Ok(json!({"count": e.prefs.rustfig.style_families.len()}))
}

fn family_delete(e: &mut Engine, p: &Value) -> CmdResult {
    let name = name_of(p, "family")?;
    let fams = &mut e.prefs.rustfig.style_families;
    let before = fams.len();
    fams.retain(|f| f.name != name);
    if fams.len() == before {
        return Err(EngineError::bad_params(format!("no style family {name:?}")));
    }
    let count = fams.len();
    e.prefs_dirty = true;
    Ok(json!({"count": count}))
}

fn family_rename(e: &mut Engine, p: &Value) -> CmdResult {
    let name = name_of(p, "family")?;
    let new = name_of(p, "new_name")?;
    family_ref(e, &name)?;
    if new != name && e.prefs.rustfig.style_families.iter().any(|f| f.name == new) {
        return Err(EngineError::bad_params(format!("a family named {new:?} exists")));
    }
    family_mut(e, &name)?.name = new;
    e.prefs_dirty = true;
    Ok(json!({"count": e.prefs.rustfig.style_families.len()}))
}

fn save(e: &mut Engine, p: &Value) -> CmdResult {
    let family = name_of(p, "family")?;
    let name = name_of(p, "name")?;
    let attrs = AttrStyle::of(&e.indicator);
    let styles = &mut family_mut(e, &family)?.styles;
    let replaced = if let Some(s) = styles.iter_mut().find(|s| s.name == name) {
        s.attrs = attrs;
        true
    } else {
        if styles.len() >= MAX_STYLES {
            return Err(EngineError::bad_params(format!(
                "at most {MAX_STYLES} styles per family"
            )));
        }
        styles.push(NamedStyle {
            name: name.clone(),
            attrs,
        });
        false
    };
    let count = styles.len();
    e.prefs_dirty = true;
    e.set_status(format!("Style \"{name}\" saved in \"{family}\""));
    Ok(json!({"count": count, "replaced": replaced}))
}

fn apply(e: &mut Engine, p: &Value) -> CmdResult {
    let family = name_of(p, "family")?;
    let name = name_of(p, "name")?;
    let attrs = find(e, &family, &name)?.attrs.clone();
    attrs.apply_to(&mut e.indicator);
    e.set_status(format!("Style \"{name}\" applied"));
    Ok(json!({}))
}

fn delete(e: &mut Engine, p: &Value) -> CmdResult {
    let family = name_of(p, "family")?;
    let name = name_of(p, "name")?;
    let styles = &mut family_mut(e, &family)?.styles;
    let before = styles.len();
    styles.retain(|s| s.name != name);
    if styles.len() == before {
        return Err(EngineError::bad_params(format!(
            "no style {name:?} in family {family:?}"
        )));
    }
    let count = styles.len();
    e.prefs_dirty = true;
    Ok(json!({"count": count}))
}

fn rename(e: &mut Engine, p: &Value) -> CmdResult {
    let family = name_of(p, "family")?;
    let name = name_of(p, "name")?;
    let new = name_of(p, "new_name")?;
    find(e, &family, &name)?;
    let overwrite = opt_bool(p, "overwrite")?.unwrap_or(false);
    let styles = &mut family_mut(e, &family)?.styles;
    if new != name && styles.iter().any(|s| s.name == new) {
        if !overwrite {
            return Err(EngineError::bad_params(format!(
                "a style named {new:?} exists (overwrite: true replaces it)"
            )));
        }
        styles.retain(|s| s.name != new);
    }
    if let Some(s) = styles.iter_mut().find(|s| s.name == name) {
        s.name = new;
    }
    let count = styles.len();
    e.prefs_dirty = true;
    Ok(json!({"count": count}))
}

fn list(e: &mut Engine, p: &Value) -> CmdResult {
    let given = |k: &str| p.get(k).is_some_and(|n| !n.is_null());
    if !given("family") {
        let fams: Vec<Value> = e
            .prefs
            .rustfig
            .style_families
            .iter()
            .map(|f| json!({"name": f.name, "count": f.styles.len()}))
            .collect();
        return Ok(json!({"families": fams}));
    }
    let family = name_of(p, "family")?;
    if given("name") {
        let name = name_of(p, "name")?;
        let s = find(e, &family, &name)?;
        let attrs = serde_json::to_value(&s.attrs).map_err(|err| EngineError::bad_params(err.to_string()))?;
        return Ok(json!({"family": family, "styles": [{"name": s.name, "attrs": attrs}]}));
    }
    let names: Vec<Value> = family_ref(e, &family)?
        .styles
        .iter()
        .map(|s| json!({"name": s.name}))
        .collect();
    Ok(json!({"family": family, "styles": names}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::Prefs;

    fn fam(e: &mut Engine, n: &str) {
        e.execute("style.family_add", &json!({"name": n})).unwrap();
    }

    #[test]
    fn family_add_delete_rename() {
        let mut e = Engine::new();
        fam(&mut e, "Report");
        assert!(e.execute("style.family_add", &json!({"name": "Report"})).is_err());
        assert!(e.execute("style.family_add", &json!({"name": " "})).is_err());
        fam(&mut e, "Poster");
        assert!(e
            .execute(
                "style.family_rename",
                &json!({"family": "Report", "new_name": "Poster"})
            )
            .is_err());
        e.execute("style.family_rename", &json!({"family": "Report", "new_name": "Paper"}))
            .unwrap();
        let l = e.execute("style.list", &json!({})).unwrap();
        assert_eq!(l["families"][0]["name"], "Paper");
        assert_eq!(l["families"].as_array().map(Vec::len), Some(2));
        assert!(e.execute("style.family_delete", &json!({"family": "Report"})).is_err());
        e.execute("style.family_delete", &json!({"family": "Paper"})).unwrap();
        assert_eq!(e.prefs.rustfig.style_families.len(), 1);
        for i in 0..MAX_FAMILIES {
            let _ = e.execute("style.family_add", &json!({"name": format!("f{i}")}));
        }
        assert_eq!(e.prefs.rustfig.style_families.len(), MAX_FAMILIES);
        assert!(e.execute("style.family_add", &json!({"name": "one more"})).is_err());
    }

    #[test]
    fn styles_live_inside_their_family_and_apply_restores_every_field() {
        let mut e = Engine::new();
        fam(&mut e, "A");
        fam(&mut e, "B");
        e.indicator.thickness = 5;
        e.indicator.pen_color = 4;
        e.indicator.fill_color = 2;
        e.indicator.area_fill = 20;
        e.indicator.forward_arrow = true;
        e.indicator.font_size = 17.0;
        e.indicator.pie_arc = true;
        e.indicator.box_radius = 9;
        e.indicator.depth = 77;
        e.indicator.dot_gap = 3.0;
        e.indicator.style_val = 4.0;
        e.execute("style.save", &json!({"family": "A", "name": " Fancy "}))
            .unwrap();
        let saved = AttrStyle::of(&e.indicator);
        assert!(e
            .execute("style.apply", &json!({"family": "B", "name": "Fancy"}))
            .is_err());
        assert!(e.execute("style.save", &json!({"family": "Z", "name": "x"})).is_err());
        e.execute("style.save", &json!({"family": "B", "name": "Fancy"}))
            .unwrap();
        e.indicator = crate::Indicator::default();
        assert_ne!(AttrStyle::of(&e.indicator), saved);
        e.execute("style.apply", &json!({"family": "A", "name": "Fancy"}))
            .unwrap();
        assert_eq!(AttrStyle::of(&e.indicator), saved);
        e.execute("style.save", &json!({"family": "A", "name": "S2"})).unwrap();
        assert!(e
            .execute(
                "style.rename",
                &json!({"family": "A", "name": "Fancy", "new_name": "S2"})
            )
            .is_err());
        e.execute(
            "style.rename",
            &json!({"family": "A", "name": "Fancy", "new_name": "C"}),
        )
        .unwrap();
        let l = e.execute("style.list", &json!({"family": "A"})).unwrap();
        assert_eq!(l["styles"].as_array().map(Vec::len), Some(2));
        let l = e.execute("style.list", &json!({"family": "A", "name": "C"})).unwrap();
        assert!(l["styles"][0]["attrs"].is_object());
        assert!(e
            .execute("style.apply", &json!({"family": "A", "name": "Fancy"}))
            .is_err());
        e.execute("style.delete", &json!({"family": "A", "name": "C"})).unwrap();
        assert!(e.execute("style.delete", &json!({"family": "A", "name": "C"})).is_err());
        assert!(e.execute("style.save", &json!({"family": "A", "name": "  "})).is_err());
        e.execute("style.family_delete", &json!({"family": "A"})).unwrap();
        assert!(e.execute("style.list", &json!({"family": "A"})).is_err());
        assert_eq!(e.prefs.rustfig.style_families[0].styles.len(), 1);
    }

    #[test]
    fn style_limit_and_prefs_round_trip() {
        let mut e = Engine::new();
        fam(&mut e, "A");
        e.indicator.thickness = 3;
        for i in 0..MAX_STYLES {
            e.execute("style.save", &json!({"family": "A", "name": format!("s{i}")}))
                .unwrap();
        }
        assert!(e
            .execute("style.save", &json!({"family": "A", "name": "extra"}))
            .is_err());
        e.execute("style.save", &json!({"family": "A", "name": "s0"})).unwrap();
        let text = e.prefs.to_toml();
        let (p, warn) = Prefs::parse(&text).unwrap();
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(p.rustfig.style_families, e.prefs.rustfig.style_families);
    }
}

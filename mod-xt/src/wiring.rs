use std::sync::{Arc, Mutex};
use pod_core::store::{Signal, Store};
use pod_core::controller::Controller;
use pod_core::model::{AbstractControl, Config};
use pod_gtk::prelude::*;
use anyhow::*;
use log::*;
use pod_core::config::{GUI, MIDI};
use pod_gtk::logic::LogicBuilder;
use crate::{config, model};
use crate::config::XtPacks;

fn is_sensitive(packs: XtPacks, name: &str) -> bool {
    let ms = name.starts_with("MS-");
    let cc = name.starts_with("CC-");
    let bx = name.starts_with("BX-");
    let fx = name.starts_with("FX-");

    (!ms && !cc && !bx && !fx) ||
        (ms && packs.contains(XtPacks::MS)) ||
        (cc && packs.contains(XtPacks::CC)) ||
        (bx && packs.contains(XtPacks::BX)) ||
        (fx && packs.contains(XtPacks::FX))
}

fn init_combo(packs: XtPacks, objs: &ObjectList, name: &str, items: Vec<&str>) -> Result<()> {
    let select = objs.ref_by_name::<gtk::ComboBox>(name)?;

    let list_store = gtk::ListStore::new(
        &[u8::static_type(), String::static_type(), bool::static_type()]
    );

    for (i, item) in items.iter().enumerate() {
        let sensitive = is_sensitive(packs, item);
        list_store.insert_with_values(None, &[
            (0, &(i as u32)), (1, item), (2, &sensitive)
        ]);
    }

    select.set_model(Some(&list_store));
    select.clear();
    select.set_entry_text_column(1);

    let renderer = gtk::CellRendererText::new();
    select.pack_start(&renderer, true);
    select.add_attribute(&renderer, "text", 1);
    select.add_attribute(&renderer, "sensitive", 2);

    Ok(())
}

pub fn init_amp_models(packs: XtPacks, objs: &ObjectList, config: &Config) -> Result<()> {
    let items = config.amp_models.iter().map(|a| a.name.as_str()).collect::<Vec<_>>();
    return init_combo(packs, objs, "amp_select", items);
}
pub fn init_cab_models(packs: XtPacks, objs: &ObjectList, config: &Config) -> Result<()> {
    let items = config.cab_models.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    return init_combo(packs, objs, "cab_select", items);
}

// todo: when switching to BX cab update the mic names from BC_MIC_NAMES!
pub fn init_mic_models(objs: &ObjectList) -> Result<()> {
    let select = objs.ref_by_name::<gtk::ComboBox>("mic_select")?;

    let list_store = gtk::ListStore::new(
        &[u8::static_type(), String::static_type(), bool::static_type()]
    );

    for (i, item) in config::MIC_NAMES.iter().enumerate() {
        list_store.insert_with_values(None, &[
            (0, &(i as u32)), (1, item), (2, &true)
        ]);
    }

    select.set_model(Some(&list_store));
    select.clear();
    select.set_entry_text_column(1);

    let renderer = gtk::CellRendererText::new();
    select.pack_start(&renderer, true);
    select.add_attribute(&renderer, "text", 1);

    Ok(())
}

pub fn wire_stomp_select(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let param_names = vec![
        "stomp_param2", "stomp_param2_wave", "stomp_param2_octave",
        "stomp_param3", "stomp_param3_octave", "stomp_param3_offset",
        "stomp_param4", "stomp_param4_offset",
        "stomp_param5", "stomp_param6",
    ];

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let objs = objs.clone();
    builder
        // wire `stomp_select` controller -> gui
        .on("stomp_select")
        .run(move |value, _, _| {
            let stomp_config = &(*config::STOMP_CONFIG)[value as usize];

            for param in param_names.iter() {
                let label_name = format!("{}_label", param);
                let label = objs.ref_by_name::<gtk::Label>(&label_name).unwrap();
                let widget = objs.ref_by_name::<gtk::Widget>(param).unwrap();

                if let Some(text) = stomp_config.labels.get(&param.to_string()) {
                    label.set_text(text);
                    label.show();
                    widget.show();
                } else {
                    label.hide();
                    widget.hide();
                }
            }
        })
        // any change on the `stomp_param2` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `stamp_param2` as a value coming from GUI
        .on("stomp_param2")
        .run(move |value, controller, _| {
            let control = controller.get_config("stomp_param2_wave").unwrap();
            let midi = control.value_from_midi(value as u8, 0);
            controller.set("stomp_param2_wave", midi, MIDI);

            let control = controller.get_config("stomp_param2_octave").unwrap();
            let midi = control.value_from_midi(value as u8, 0);
            controller.set("stomp_param2_octave", midi, MIDI);
        })
        .on("stomp_param2_wave").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param2_wave").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("stomp_param2", midi as u16, origin);
        })
        .on("stomp_param2_octave").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param2_octave").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("stomp_param2", midi as u16, origin);
        })
        // any change on the `stomp_param3` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `stamp_param3` as a value coming from GUI
        .on("stomp_param3")
        .run(move |value, controller, _| {
            let control = controller.get_config("stomp_param3_octave").unwrap();
            let midi = control.value_from_midi(value as u8, 0);
            controller.set("stomp_param3_octave", midi, MIDI);

            let control = controller.get_config("stomp_param3_offset").unwrap();
            let midi = control.value_from_midi(value as u8, 0);
            controller.set("stomp_param3_offset", midi, MIDI);
        })
        .on("stomp_param3_octave").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param3_octave").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("stomp_param3", midi as u16, origin);
        })
        .on("stomp_param3_offset").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param3_offset").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("stomp_param3", midi as u16, origin);
        })
        // any change on the `stomp_param4` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `stamp_param4` as a value coming from GUI
        .on("stomp_param4")
        .run(move |value, controller, _| {
            let control = controller.get_config("stomp_param4_offset").unwrap();
            let midi = control.value_from_midi(value as u8, 0);
            controller.set("stomp_param4_offset", midi, MIDI);
        })
        .on("stomp_param4_offset").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("stomp_param4_offset").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("stomp_param4", midi as u16, origin);
        });

    Ok(())
}

pub fn wire_mod_select(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let param_names = vec!["mod_param2", "mod_param3", "mod_param4"];

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let objs = objs.clone();
    builder
        // wire `mod_select` controller -> gui
        .on("mod_select")
        .run(move |value, _, _| {
            let mod_config = &(*config::MOD_CONFIG)[value as usize];

            for param in param_names.iter() {
                let label_name = format!("{}_label", param);
                let label = objs.ref_by_name::<gtk::Label>(&label_name).unwrap();
                let widget = objs.ref_by_name::<gtk::Widget>(param).unwrap();

                if let Some(text) = mod_config.labels.get(&param.to_string()) {
                    label.set_text(text);
                    label.show();
                    widget.show();
                } else {
                    label.hide();
                    widget.hide();
                }
            }
        });

    Ok(())
}

pub fn wire_delay_select(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks) -> Result<()> {
    let param_names = vec![
        "delay_param2",
        "delay_param3", "delay_param3_heads",
        "delay_param4", "delay_param4_bits",
    ];

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let objs = objs.clone();
    builder
        // wire `delay_select` controller -> gui
        .on("delay_select")
        .run(move |value, _, _| {
            let config = &(*config::DELAY_CONFIG)[value as usize];

            for param in param_names.iter() {
                let label_name = format!("{}_label", param);
                let label = objs.ref_by_name::<gtk::Label>(&label_name).unwrap();
                let widget = objs.ref_by_name::<gtk::Widget>(param).unwrap();

                if let Some(text) = config.labels.get(&param.to_string()) {
                    label.set_text(text);
                    label.show();
                    widget.show();
                } else {
                    label.hide();
                    widget.hide();
                }
            }
        })
        // any change on the `delay_param3` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `delay_param3` as a value coming from GUI
        .on("delay_param3")
        .run(move |value, controller, _| {
            let control = controller.get_config("delay_param3_heads").unwrap();
            let midi = control.value_from_midi(value as u8, 0);
            controller.set("delay_param3_heads", midi, MIDI);
        })
        .on("delay_param3_heads").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("delay_param3_heads").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("delay_param3", midi as u16, origin);
        })
        // any change on the `delay_param4` will show up on the virtual
        // controls as a value coming from MIDI, GUI changes from virtual
        // controls will show up on `stamp_param4` as a value coming from GUI
        .on("delay_param4")
        .run(move |value, controller, _| {
            let control = controller.get_config("delay_param4_bits").unwrap();
            let midi = control.value_from_midi(value as u8, 0);
            controller.set("delay_param4_bits", midi, MIDI);
        })
        .on("delay_param4_bits").from(GUI)
        .run(move |value, controller, origin| {
            let control = controller.get_config("delay_param4_bits").unwrap();
            let midi = control.value_to_midi(value);
            controller.set("delay_param4", midi as u16, origin);
        });

    Ok(())
}

pub fn wire_14bit(controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks,
                  control_name: &str, msb_name: &str, lsb_name: &str) -> Result<()> {
    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    let objs = objs.clone();
    builder
        .on(control_name)
        .run({
            let lsb_name = lsb_name.to_string();
            let msb_name = msb_name.to_string();

            move |value, controller, origin| {
                let msb = (value & 0x3f80) >> 7;
                let lsb = value & 0x7f;

                // Make sure GUI event always generates both MSB and LSB MIDI messages
                let signal = if origin == GUI { Signal::Force } else { Signal::Change };
                controller.set_full(&msb_name, msb, origin, signal.clone());
                controller.set_full(&lsb_name, lsb, origin, signal);
            }
        })
        .on(msb_name).from(MIDI)
        .run({
            let control_name = control_name.to_string();

            move |value, controller, origin| {
                let control_value = controller.get(&control_name).unwrap();
                let lsb = control_value & 0x7f;
                let control_value = ((value & 0x7f) << 7) | lsb;
                controller.set(&control_name, control_value, origin);
            }
        })
        .on(lsb_name).from(MIDI)
        .run({
            let control_name = control_name.to_string();

            move |value, controller, origin| {
                let control_value = controller.get(&control_name).unwrap();
                let msb = control_value & 0x3f80;
                let control_value = msb | (value & 0x7f);
                controller.set(&control_name, control_value, origin);
            }
        });

    Ok(())
}

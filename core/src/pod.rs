use midir::*;
use anyhow::{Result, Context};
use regex::Regex;
use std::str::FromStr;
use std::time::Duration;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use tokio::time::sleep;
use log::*;
use result::prelude::*;

use crate::midi::*;
use crate::config::configs;
use crate::model::Config;
use crate::util::OptionToResultsExt;
use tokio::sync::mpsc;

pub struct MidiIn {
    pub name: String,
    port: MidiInputPort,
    conn: MidiInputConnection<()>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>
}

impl MidiIn {
    fn _new() -> Result<MidiInput> {
        let mut midi_in = MidiInput::new("pod midi in")?;
        midi_in.ignore(Ignore::None);

        for (i, port) in midi_in.ports().iter().enumerate() {
            debug!("midi in {}: {:?}", i, midi_in.port_name(port)?);
        }

        Ok(midi_in)
    }

    pub fn _new_for_port(midi_in: MidiInput, port: MidiInputPort) -> Result<Self> {
        let name = midi_in.port_name(&port)
            .map_err(|e| anyhow!("Failed to get MIDI intput port name: {}", e))?;

        let (tx, rx) = mpsc::unbounded_channel();

        let conn = midi_in.connect(&port, "pod midi in conn", move |ts, data, _| {
            trace!("<< {:02x?} len={} ts={}", data, data.len(), ts);
            tx.send(Vec::from(data)).unwrap();

        }, ())
            .map_err(|e| anyhow!("Midi connection error: {:?}", e))?;

        Ok(MidiIn { name, port, conn, rx })
    }

    pub async fn recv(&mut self) -> Option<Vec<u8>>
    {
        self.rx.recv().await
    }
}


pub struct MidiOut {
    pub name: String,
    port: MidiOutputPort,
    conn: MidiOutputConnection,
}

impl MidiOut {
    fn _new() -> Result<MidiOutput> {
        let midi_out = MidiOutput::new("pod midi out")?;

        for (i, port) in midi_out.ports().iter().enumerate() {
            debug!("midi out {}: {:?}", i, midi_out.port_name(port)?);
        }

        Ok(midi_out)
    }

    fn _new_for_port(midi_out: MidiOutput, port: MidiOutputPort) -> Result<Self> {
        let name = midi_out.port_name(&port)
            .map_err(|e| anyhow!("Failed to get MIDI output port name: {}", e))?;
        let conn = midi_out.connect(&port, "pod midi out conn")
            .map_err(|e| anyhow!("Midi connection error: {:?}", e))?;

        Ok(MidiOut { name, port, conn })
    }

    pub fn send(&mut self, bytes: &[u8]) -> Result<()> {
        trace!(">> {:02x?} len={}", bytes, bytes.len());
        self.conn.send(bytes)
            .map_err(|e| anyhow!("Midi send error: {:?}", e))
    }
}

pub trait  MidiOpen {
    type Class: MidiIO<Port = Self::Port>;
    type Port;
    type Out;
    const DIR: &'static str;

    fn _new() -> Result<Self::Class>;
    fn _new_for_port(class: Self::Class, port: Self::Port) -> Result<Self::Out>;

    fn new(port_idx: Option<usize>) -> Result<Self::Out> {
        let class = Self::_new()?;

        let port_n: usize = port_idx.unwrap_or(0);
        let port = class.ports().into_iter().nth(port_n)
            .with_context(|| format!("MIDI {} port {} not found", Self::DIR, port_n))?;

        Self::_new_for_port(class, port)
    }

    fn new_for_address(port_addr: String) -> Result<Self::Out> {
        let class = Self::_new()?;

        let port_n_re = Regex::new(r"\d+").unwrap();
        let port_id_re = Regex::new(r"\d+:\d+").unwrap();

        let mut found = None;
        if port_id_re.is_match(&port_addr) {
            for port in class.ports().into_iter() {
                let name = class.port_name(&port)?;
                if name.ends_with(&port_addr) {
                    found = Some(port);
                }
            }
        } else if port_n_re.is_match(&port_addr) {
            let n = Some(usize::from_str(&port_addr)).invert()
                .with_context(|| format!("Unrecognized MIDI port index {:?}", port_addr))?;
            return Self::new(n);
        } else {
            bail!("Unrecognized MIDI port address {:?}", port_addr);
        }

        if found.is_none() {
            bail!("MIDI {} port for address {:?} not found!", Self::DIR, port_addr);
        }

        Self::_new_for_port(class, found.unwrap())
    }

    fn new_for_name(port_name: &str) -> Result<Self::Out> {
        let class = Self::_new()?;

        let mut found = None;
        for port in class.ports().into_iter() {
            let name = class.port_name(&port)?;
            if name == port_name {
                found = Some(port);
            }
        }
        if found.is_none() {
            bail!("MIDI {} port for name {:?} not found!", Self::DIR, port_name);
        }

        Self::_new_for_port(class, found.unwrap())
    }
}

impl MidiOpen for MidiIn {
    type Class = MidiInput;
    type Port = MidiInputPort;
    type Out = MidiIn;
    const DIR: &'static str = "input";

    fn _new() -> Result<Self::Class> {
        MidiIn::_new()
    }

    fn _new_for_port(class: Self::Class, port: Self::Port) -> Result<Self::Out> {
        MidiIn::_new_for_port(class, port)
    }
}

impl MidiOpen for MidiOut {
    type Class = MidiOutput;
    type Port = MidiOutputPort;
    type Out = MidiOut;
    const DIR: &'static str = "output";

    fn _new() -> Result<Self::Class> {
        MidiOut::_new()
    }

    fn _new_for_port(class: Self::Class, port: Self::Port) -> Result<Self::Out> {
        MidiOut::_new_for_port(class, port)
    }
}


pub trait MidiPorts {
    fn all_ports() -> Result<Vec<String>>;
    fn ports() -> Result<Vec<String>>;
}

impl MidiPorts for MidiIn {
    fn all_ports() -> Result<Vec<String>> {
        let midi = MidiIn::_new()?;
        list_ports(midi)
    }

    fn ports() -> Result<Vec<String>> {
        Self::all_ports()
            .map(|v| v.into_iter()
                .filter(|name| !name.starts_with("pod midi out:"))
                .collect()
            )
    }
}

impl MidiPorts for MidiOut {
    fn all_ports() -> Result<Vec<String>> {
        let midi = MidiOut::_new()?;
        list_ports(midi)
    }

    fn ports() -> Result<Vec<String>> {
        Self::all_ports()
            .map(|v| v.into_iter()
                .filter(|name| !name.starts_with("pod midi in:"))
                .collect()
            )
    }
}

fn list_ports<T: midir::MidiIO>(midi: T) -> Result<Vec<String>> {
    let port_names: Result<Vec<_>, _> =
        midi.ports().iter()
            .map(|port| midi.port_name(port))
            .collect::<Result<Vec<_>, _>>();
    port_names.map_err(|err| anyhow!("Error getting port names: {}", err))
}

fn find_address<'a>(addresses: impl Iterator<Item = &'a str>, id: &'a str) -> Result<Option<usize>> {
    let port_n_re = Regex::new(r"\d+").unwrap();
    let port_id_re = Regex::new(r"\d+:\d+").unwrap();

    if port_id_re.is_match(id) {
        for (i, n) in addresses.enumerate() {
            if n.ends_with(id) {
                return Ok(Some(i));
            }
        }
        bail!("MIDI device with address {:?} not found", id);
    } else if port_n_re.is_match(id) {
        return Ok(Some(usize::from_str(id).unwrap()));
    }

    bail!("Failed to parse {:?} as a MIDI device address or index", id)
}
/*
pub struct PodConfigs {
}

impl PodConfigs {
    pub fn new() -> Result<Self> {
        Ok(PodConfigs {})
    }

    pub fn count(&self) -> usize {
        PODS.len()
    }

    pub fn by_name(&self, name: &String) -> Option<Config> {
        PODS.iter().find(|config| &config.name == name).map(|c| c.clone())
    }

    /*
    pub fn detect(&self, midi: &mut Midi) -> Result<&Config> {
        midi.send(MidiMessage::UniversalDeviceInquiry { channel: Channel::all() }.to_bytes().as_slice())?;
        midi.recv(move |_ts, data| {
            let event = MidiResponse::from_bytes(data)?;
            match event {
                MidiResponse::UniversalDeviceInquiry { channel: _, family, member, ver: _ } => {
                    let pod = PODS().iter().find(|config| {
                        family == config.family && member == config.member
                    }).unwrap();
                    info!("Discovered: {}", pod.name);
                    Ok(pod)
                }
                _ => Err(anyhow!("Incorrect MIDI response"))
            }
        })
    }

    pub fn dump_all(&self, midi: &mut Midi, config: &Config) -> Result<Vec<u8>> {
        midi.send(MidiMessage::AllProgramsDumpRequest.to_bytes().as_slice())?;
        midi.recv(move |_ts, data| {
            let event = MidiResponse::from_bytes(data)?;
            match event {
                MidiResponse::AllProgramsDump { ver: _, data } => {
                    if data.len() == config.all_programs_size {
                        Ok(data)
                    } else {
                        error!("Program size mismatch: expected {}, got {}", config.all_programs_size, data.len());
                        Err(anyhow!("Program size mismatch"))
                    }
                }
                _ => Err(anyhow!("Incorrect MIDI response"))
            }
        })
    }

    pub fn dump_edit(&self, midi: &mut Midi, config: &Config) -> Result<Vec<u8>> {
        midi.send(MidiMessage::ProgramEditBufferDumpRequest.to_bytes().as_slice())?;
        midi.recv(move |_ts, data| {
            let event = MidiResponse::from_bytes(data)?;
            match event {
                MidiResponse::ProgramEditBufferDump { ver: _, data } => {
                    if data.len() == config.program_size {
                        Ok(data)
                    } else {
                        error!("Program size mismatch: expected {}, got {}", config.program_size, data.len());
                        Err(anyhow!("Program size mismatch"))
                    }
                }
                _ => Err(anyhow!("Incorrect MIDI response"))
            }
        })
    }

     */
}

 */

const DETECT_DELAY: Duration = Duration::from_millis(1000);

async fn detect(in_ports: &mut [MidiIn], out_ports: &mut [MidiOut]) -> Result<Vec<usize>> {

    let udi = MidiMessage::UniversalDeviceInquiry { channel: Channel::all() }.to_bytes();

    let mut futures = FuturesUnordered::new();
    for (i, p) in in_ports.into_iter().enumerate() {
        futures.push(async move {
            p.rx.recv().await.map(|v| (i, v))
        })
    }
    let mut delay = Box::pin(sleep(DETECT_DELAY));

    for p in out_ports {
        p.send(&udi)?;
    }

    let mut replied_midi_in = Vec::<usize>::new();
    loop {
        tokio::select! {
            Some(Some((i, bytes))) = futures.next() => {
                let event = MidiMessage::from_bytes(bytes).ok();
                let found = match event {
                    Some(MidiMessage::UniversalDeviceInquiryResponse { family, member, .. }) => {
                        let pod: Option<&Config> = configs().iter().find(|config| {
                            family == config.family && member == config.member
                        });
                        pod.map(|pod| {
                            info!("Discovered: {}: {}", i, pod.name);
                            true
                        }).or_else(|| {
                            info!("Discovered unknown device: {}: {}/{}, skipping!", i, family, member);
                            Some(false)
                        }).unwrap()
                    },
                    _ => false
                };

                if found {
                    replied_midi_in.push(i);
                }
            },
            _ = &mut delay => { break; }
        }
    }

    Ok(replied_midi_in)
}

pub async fn autodetect() -> Result<(MidiIn, MidiOut)> {
    let in_port_names = MidiIn::ports()?;
    let mut in_ports = in_port_names.iter().enumerate()
        .map(|(i, _)| MidiIn::new(Some(i)))
        .collect::<Result<Vec<_>>>()?;

    let out_port_names = MidiOut::ports()?;
    let mut out_ports = out_port_names.iter().enumerate()
        .map(|(i, _)| MidiOut::new(Some(i)))
        .collect::<Result<Vec<_>>>()?;

    if in_ports.len() < 1 {
        bail!("No MIDI input ports found")
    }
    if out_ports.len() < 1 {
        bail!("No MIDI output ports found")
    }

    // 1. find the input
    {
        let rep = detect(in_ports.as_mut_slice(), out_ports.as_mut_slice()).await?;
        if rep.len() == 0 {
            bail!("Received no device response");
        }
        if rep.len() == in_ports.len() {
            bail!("Received device response on multiple ({}) ports", rep.len());
        }
        in_ports = in_ports.into_iter().enumerate()
            .filter(|(i, _)| *i == rep[0]).map(|(_,v)| v).collect();
    }

    // 2. find the output
    loop {
        let slice =  (out_ports.len() as f32 / 2.0).ceil() as usize;
        println!("len {} slice {}", out_ports.len(), slice);
        let chunks = out_ports.chunks_mut(slice);
        let mut good = Vec::<usize>::new();
        let mut i = 0usize;
        for chunk in chunks {
            let rep = detect(in_ports.as_mut_slice(), chunk).await?;
            if rep.len() > 0 {
                for x in i .. i+chunk.len() {
                    good.push(x);
                }
                // binary search: this group is good, let's continue with it!
                break;
            }
            i += chunk.len();
        }
        if good.len() == 0 {
            bail!("Received no device response (output search)");
        }
        if good.len() == out_ports.len() {
            bail!("Can't determine output port -- stuck at {}", good.len());
        }
        out_ports = out_ports.into_iter().enumerate()
            .filter(|(i, _)| good.contains(i)).map(|(_,v)| v).collect();
        if out_ports.len() == 1 {
            break;
        }
    }

    Ok((in_ports.remove(0), out_ports.remove(0)))
}

pub async fn test(in_name: &str, out_name: &str) -> Result<(MidiIn, MidiOut)> {
    let in_port = MidiIn::new_for_name(in_name)?;
    let out_port = MidiOut::new_for_name(out_name)?;
    let mut in_ports = vec![in_port];
    let mut out_ports = vec![out_port];

    let rep = detect(in_ports.as_mut_slice(), out_ports.as_mut_slice()).await?;
    if rep.len() == 0 {
        bail!("Received no device response");
    }

    Ok((in_ports.remove(0), out_ports.remove(0)))
}

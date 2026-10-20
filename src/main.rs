use num_bigint::BigUint;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn sha256d(data: &[u8]) -> [u8; 32] {
    let first = Sha256::digest(data);
    let second = Sha256::digest(first);
    let mut out = [0u8; 32];
    out.copy_from_slice(&second);
    out
}
fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 { return Err("odd-length hex string".into()); }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i+2],16).map_err(|e|e.to_string())).collect()
}
fn encode_hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }
fn reverse_bytes(bytes: &mut [u8]) { bytes.reverse(); }
fn swap_u32_words(bytes: &mut [u8]) -> Result<(), String> {
    if bytes.len()%4 != 0 { return Err("prevhash must contain 4-byte words".into()); }
    for word in bytes.chunks_exact_mut(4) { word.reverse(); }
    Ok(())
}
fn parse_u32_hex(s: &str, field: &str) -> Result<u32,String> {
    u32::from_str_radix(s,16).map_err(|e|format!("invalid {field}: {e}"))
}
fn difficulty_target(difficulty: f64) -> Result<[u8;32],String> {
    if !difficulty.is_finite() || difficulty <= 0.0 { return Err("pool difficulty must be positive".into()); }
    let diff1 = BigUint::parse_bytes(b"00000000ffff0000000000000000000000000000000000000000000000000000",16).ok_or("target parse failed")?;
    let scaled = (&diff1 * BigUint::from(1_000_000_000u64)) / BigUint::from((difficulty*1_000_000_000.0).max(1.0) as u64);
    let bytes = scaled.to_bytes_be();
    if bytes.len()>32 { return Err("share target exceeds 256 bits".into()); }
    let mut target=[0u8;32]; target[32-bytes.len()..].copy_from_slice(&bytes); Ok(target)
}
fn hash_meets_target(hash_internal:&[u8;32],target_be:&[u8;32])->bool {
    let mut display=*hash_internal; reverse_bytes(&mut display); display.as_slice()<=target_be.as_slice()
}
#[derive(Clone)]
struct Job { id:String, prevhash:String, coinb1:String, coinb2:String, branches:Vec<String>, version:String, nbits:String, ntime:String }
fn parse_job(p:&[Value])->Result<Job,String>{
    if p.len()<9{return Err("mining.notify has fewer than 9 parameters".into());}
    let get=|i:usize|p.get(i).and_then(Value::as_str).map(str::to_owned).ok_or_else(||format!("parameter {i} is not a string"));
    let branches=p[4].as_array().ok_or("merkle branches not an array")?.iter().map(|v|v.as_str().map(str::to_owned).ok_or("branch not a string")).collect::<Result<Vec<_>,_>>()?;
    Ok(Job{id:get(0)?,prevhash:get(1)?,coinb1:get(2)?,coinb2:get(3)?,branches,version:get(5)?,nbits:get(6)?,ntime:get(7)?})
}
fn build_header(job:&Job,ex1:&str,ex2:&[u8],nonce:u32)->Result<[u8;80],String>{
    let mut coinbase=decode_hex(&job.coinb1)?; coinbase.extend(decode_hex(ex1)?); coinbase.extend_from_slice(ex2); coinbase.extend(decode_hex(&job.coinb2)?);
    let mut merkle=sha256d(&coinbase);
    for item in &job.branches { let mut branch=decode_hex(item)?; if branch.len()!=32{return Err("merkle branch must be 32 bytes".into());} reverse_bytes(&mut branch); let mut pair=Vec::with_capacity(64); pair.extend_from_slice(&merkle); pair.extend_from_slice(&branch); merkle=sha256d(&pair); }
    let mut prev=decode_hex(&job.prevhash)?; if prev.len()!=32{return Err("prevhash must be 32 bytes".into());} swap_u32_words(&mut prev)?;
    let mut h=[0u8;80]; h[0..4].copy_from_slice(&parse_u32_hex(&job.version,"version")?.to_le_bytes()); h[4..36].copy_from_slice(&prev); h[36..68].copy_from_slice(&merkle); h[68..72].copy_from_slice(&parse_u32_hex(&job.ntime,"ntime")?.to_le_bytes()); h[72..76].copy_from_slice(&parse_u32_hex(&job.nbits,"nbits")?.to_le_bytes()); h[76..80].copy_from_slice(&nonce.to_le_bytes()); Ok(h)
}
fn send_json(socket:&Arc<Mutex<TcpStream>>,v:&Value)->io::Result<()> { let mut s=socket.lock().map_err(|_|io::Error::other("socket lock poisoned"))?; serde_json::to_writer(&mut *s,v)?; s.write_all(b"\n")?; s.flush() }
fn mine_job(job:Job,user:String,ex1:String,ex2_size:usize,target:[u8;32],socket:Arc<Mutex<TcpStream>>,cancel:Arc<AtomicBool>){
    thread::spawn(move||{
        let ex2=vec![0u8;ex2_size]; let mut nonce=0u32; let mut count=0u64;
        loop {
            if cancel.load(Ordering::Relaxed){break;}
            let header=match build_header(&job,&ex1,&ex2,nonce){Ok(h)=>h,Err(e)=>{eprintln!("Header error: {e}");break;}};
            let hash=sha256d(&header); count+=1;
            if hash_meets_target(&hash,&target){
                let req=json!({"id":4,"method":"mining.submit","params":[user.clone(),job.id.clone(),encode_hex(&ex2),job.ntime.clone(),format!("{nonce:08x}")]});
                if let Err(e)=send_json(&socket,&req){eprintln!("Submit failed: {e}");break;}
                println!("Submitted candidate share for job {}",job.id);
            }
            if count%1_000_000==0 { println!("Job {}: {} hashes attempted",job.id,count); }
            if nonce==u32::MAX {break;} nonce=nonce.wrapping_add(1);
        }
    });
}
fn run()->Result<(),String>{
    let host=env::var("BTC_POOL_HOST").map_err(|_|"Set BTC_POOL_HOST to your pool host".to_string())?;
    let port=env::var("BTC_POOL_PORT").unwrap_or_else(|_|"3333".into());
    let user=env::var("BTC_POOL_USER").map_err(|_|"Set BTC_POOL_USER to your pool username/worker".to_string())?;
    let pass=env::var("BTC_POOL_PASSWORD").unwrap_or_else(|_|"x".into());
    println!("KritikBTCMiner experimental Stratum V1 CPU client");
    println!("Connecting to {host}:{port}; pool username={user}");
    println!("Bitcoin CPU mining on a phone is not expected to earn meaningful BTC.");
    let stream=TcpStream::connect(format!("{host}:{port}")).map_err(|e|format!("connect failed: {e}"))?;
    stream.set_nodelay(true).map_err(|e|e.to_string())?;
    let socket=Arc::new(Mutex::new(stream.try_clone().map_err(|e|e.to_string())?));
    let mut reader=BufReader::new(stream);
    send_json(&socket,&json!({"id":1,"method":"mining.subscribe","params":["KritikBTCMiner/0.2"]})).map_err(|e|e.to_string())?;
    let mut line=String::new(); let mut ex1=String::new(); let mut ex2_size=0usize; let mut subscribed=false; let mut authorized=false; let mut diff=1.0f64; let mut cancel:Option<Arc<AtomicBool>>=None;
    loop{
        line.clear(); if reader.read_line(&mut line).map_err(|e|e.to_string())?==0{return Err("pool closed connection".into());}
        let m:Value=match serde_json::from_str(line.trim()){Ok(v)=>v,Err(e)=>{eprintln!("Bad JSON from pool: {e}");continue;}};
        if let Some(method)=m.get("method").and_then(Value::as_str){
            let p=m.get("params").and_then(Value::as_array).cloned().unwrap_or_default();
            match method{
                "mining.set_difficulty"=>if let Some(d)=p.first().and_then(Value::as_f64){diff=d;println!("Pool difficulty: {diff}");},
                "mining.set_extranonce"=>{if p.len()>=2{if let Some(s)=p[0].as_str(){ex1=s.into();}if let Some(n)=p[1].as_u64(){ex2_size=n as usize;}if let Some(c)=&cancel{c.store(true,Ordering::Relaxed);}}},
                "mining.notify"=>{
                    if !subscribed||!authorized{eprintln!("Ignoring job before authorization");continue;}
                    let job=parse_job(&p)?; if let Some(c)=&cancel{c.store(true,Ordering::Relaxed);}
                    let target=difficulty_target(diff)?;let c=Arc::new(AtomicBool::new(false));cancel=Some(c.clone());
                    println!("New pool job: {}",job.id);
                    mine_job(job,user.clone(),ex1.clone(),ex2_size,target,socket.clone(),c);
                },
                _=>println!("Pool notification: {method}")
            } continue;
        }
        match m.get("id").and_then(Value::as_i64){
            Some(1)=>{
                if m.get("error").is_some_and(|e|!e.is_null()){return Err(format!("subscribe rejected: {}",m["error"]));}
                let r=m.get("result").and_then(Value::as_array).ok_or("malformed subscribe response")?;
                if r.len()<3{return Err("subscribe response missing extranonce".into());}
                ex1=r[1].as_str().ok_or("invalid extranonce1")?.into();ex2_size=r[2].as_u64().ok_or("invalid extranonce2_size")? as usize;
                if ex2_size==0||ex2_size>16{return Err("unsupported extranonce2 size".into());}
                subscribed=true;send_json(&socket,&json!({"id":2,"method":"mining.authorize","params":[user,pass]})).map_err(|e|e.to_string())?;println!("Subscribed; authorizing.");
            },
            Some(2)=>{authorized=m.get("result").and_then(Value::as_bool).unwrap_or(false);if !authorized{return Err(format!("pool authorization failed: {}",m.get("error").unwrap_or(&Value::Null)));}println!("Pool authorization accepted.");},
            Some(4)=>{if m.get("result").and_then(Value::as_bool).unwrap_or(false){println!("POOL ACCEPTED SHARE");}else{eprintln!("POOL REJECTED SHARE: {}",m.get("error").unwrap_or(&Value::Null));}},
            _=>{}
        }
    }
}
fn main(){if let Err(e)=run(){eprintln!("Error: {e}");std::process::exit(1);}}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn sha256d_is_32_bytes(){assert_eq!(sha256d(b"abc").len(),32);}
    #[test]fn hex_round_trip(){let b=vec![0,1,0xab,0xff];assert_eq!(decode_hex(&encode_hex(&b)).unwrap(),b);}
    #[test]fn diff_one_target_prefix(){let t=difficulty_target(1.0).unwrap();assert_eq!(&t[..6],&[0,0,0,0,0xff,0xff]);}
    #[test]fn zero_hash_meets_easy_target(){assert!(hash_meets_target(&[0;32],&[0xff;32]));}
}

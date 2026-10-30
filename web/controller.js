"use strict";
const keyMap = {KeyW:2,ArrowUp:2,KeyS:128,ArrowDown:128,KeyA:4,ArrowLeft:4,
  KeyD:8,ArrowRight:8,Space:1,ControlLeft:1,ControlRight:1,KeyE:16,Escape:32,Enter:64,NumpadEnter:64};
const pressed = new Map();
let token = "", armed = false, stopped = false, sending = false;
let pending = [];
let screenSnapshot = 0;
let connectionId = null, usbConnected = false;
let lcd = null, displayBusy = false, pacingBusy = false;
let mockupTurns = 0, lastLcdRotation = 0;
let rotationMode = "device";
const $ = id => document.getElementById(id);
function mask() { let value = 0; for (const bit of pressed.values()) value |= bit; return value; }
function paint() {
  const value = mask();
  document.querySelectorAll("[data-mask]").forEach(button => {
    const active = !!(value & Number(button.dataset.mask));
    button.classList.toggle("active",active); button.setAttribute("aria-pressed",String(active));
  });
  $("focus").textContent = armed ? "Keyboard active · keep this tab focused" : "Click Enable keyboard to play";
  $("enable").textContent = armed ? "Keyboard enabled" : "Enable keyboard";
}
async function sendPending() {
  if (sending || !token || stopped) return;
  sending = true;
  try {
    while (pending.length && !stopped) {
      const value = pending.shift();
      const response = await fetch("/api/keys",{method:"POST",headers:{"Content-Type":"application/json","X-APDO-Token":token},body:JSON.stringify({buttons:value}),keepalive:true});
      if (!response.ok) throw new Error("Waiting for USB reconnection. Enable keyboard again once connected.");
    }
  } catch (error) { pending=[]; pressed.clear(); armed=false; paint(); if(!stopped) $("error").textContent=error.message; }
  finally { sending = false; }
}
function queue() { const value=mask(); if (pending.at(-1)!==value) pending.push(value); paint(); void sendPending(); }
function releaseAll() {
  const ownedInput=armed || pressed.size>0 || pending.some(value=>value!==0);
  pressed.clear(); armed=false; pending=[];
  if(ownedInput) queue(); else paint();
}
function paintMockup() {
  const mockup=document.querySelector(".device-mockup");
  // The LCD rotates inside the meter. To show an upright game, the enclosure
  // must turn the opposite way around the fixed, calibrated screen center.
  mockup.style.setProperty("--body-turn",`${-mockupTurns*90}deg`);
  mockup.style.setProperty("--video-turn",`${rotationMode==="whole" ? -mockupTurns*90 : 0}deg`);
}
$("rotation-mode").addEventListener("change",()=>{
  rotationMode=$("rotation-mode").value;
  releaseAll(); paintMockup();
});
function paintLcd(connected) {
  if(lcd) {
    mockupTurns += (lcd.rotation-lastLcdRotation+6)%4-2;
    lastLcdRotation=lcd.rotation;
    paintMockup();
  }
  $("brightness").textContent=lcd ? `${lcd.brightness}%` : "—";
  $("rotation").textContent=lcd ? `${lcd.rotation*90}°` : "—";
  for(const id of ["dim","brighten","rotate-left","rotate-right"]) $(id).disabled=stopped || !connected || !lcd || displayBusy;
  if(lcd) { $("dim").disabled ||= lcd.brightness<=10; $("brighten").disabled ||= lcd.brightness>=100; }
}
async function setLcd(setting,value) {
  if(displayBusy || !lcd) return;
  releaseAll(); displayBusy=true; paintLcd(true);
  try {
    const response=await fetch("/api/display",{method:"POST",headers:{"Content-Type":"application/json","X-APDO-Token":token},body:JSON.stringify({setting,value})});
    if(!response.ok) throw new Error("Could not send LCD setting");
    // Wait for the device acknowledgement, not just the HTTP enqueue response.
    const deadline=Date.now()+4000;
    while(Date.now()<deadline) {
      await new Promise(resolve=>setTimeout(resolve,150));
      const state=await (await fetch("/api/status")).json();
      if(state.display_error) throw new Error(state.display_error);
      if(state.display && state.display[setting]===value) { lcd=state.display; return; }
    }
    throw new Error("The meter did not acknowledge the LCD setting");
  } catch(error) { $("display-error").textContent=error.message; }
  finally { displayBusy=false; paintLcd(true); }
}
$("pacing").addEventListener("change",async()=>{
  const enabled=$("pacing").value==="1";
  releaseAll(); pacingBusy=true; $("pacing").disabled=true;
  try {
    const response=await fetch("/api/pacing",{method:"POST",headers:{"Content-Type":"application/json","X-APDO-Token":token},body:JSON.stringify({enabled})});
    if(!response.ok) throw new Error("Could not change frame pacing");
    const deadline=Date.now()+4000;
    while(Date.now()<deadline) {
      await new Promise(resolve=>setTimeout(resolve,150));
      const state=await (await fetch("/api/status")).json();
      if(state.pacing_error) throw new Error(state.pacing_error);
      if(state.pacing && state.pacing.smooth===enabled) return;
    }
    throw new Error("The meter did not acknowledge frame pacing");
  } catch(error) { $("pacing-error").textContent=error.message; }
  finally { pacingBusy=false; }
});
$("dim").addEventListener("click",()=>void setLcd("brightness",Math.max(10,lcd.brightness-10)));
$("brighten").addEventListener("click",()=>void setLcd("brightness",Math.min(100,lcd.brightness+10)));
$("rotate-left").addEventListener("click",()=>void setLcd("rotation",(lcd.rotation+3)%4));
$("rotate-right").addEventListener("click",()=>void setLcd("rotation",(lcd.rotation+1)%4));
$("enable").disabled=true;
$("enable").addEventListener("click",()=>{if(usbConnected) {armed=true;paint();}});
$("release").addEventListener("click",releaseAll);
$("preview-enabled").addEventListener("change",async()=>{
  try {
    const response=await fetch("/api/preview",{method:"POST",headers:{"Content-Type":"application/json","X-APDO-Token":token},body:JSON.stringify({enabled:$("preview-enabled").checked})});
    if(!response.ok) throw new Error("Could not change preview state");
  } catch(error) { $("screen-error").textContent=error.message; }
});
window.addEventListener("keydown",event=>{
  if (!armed || !(event.code in keyMap)) return;
  event.preventDefault();
  if (!pressed.has(event.code)) { pressed.set(event.code,keyMap[event.code]); queue(); }
});
window.addEventListener("keyup",event=>{
  if (!(event.code in keyMap) || !pressed.has(event.code)) return;
  if (armed) event.preventDefault();
  pressed.delete(event.code); queue();
});
window.addEventListener("blur",releaseAll);
window.addEventListener("pagehide",releaseAll);
document.addEventListener("visibilitychange",()=>{if(document.hidden) releaseAll();});
document.querySelectorAll("[data-mask]").forEach(button=>{
  button.addEventListener("pointerdown",event=>{
    if(!usbConnected) return;
    event.preventDefault(); armed=true; button.setPointerCapture(event.pointerId);
    pressed.set("pointer"+event.pointerId,Number(button.dataset.mask)); queue();
  });
  const release=event=>{if(pressed.delete("pointer"+event.pointerId)) queue();};
  button.addEventListener("pointerup",release);button.addEventListener("pointercancel",release);
  button.addEventListener("lostpointercapture",release);
});
$("stop").addEventListener("click",async()=>{
  stopped=true; releaseAll(); pending=[];
  try { await fetch("/api/stop",{method:"POST",headers:{"Content-Type":"application/json","X-APDO-Token":token},body:"{}"}); }
  finally { $("connection").textContent="Controller stopped"; $("dot").classList.remove("connected"); $("error").textContent=""; document.querySelectorAll("button").forEach(button=>{button.disabled=true;}); }
});
async function status() {
  if (stopped) return;
  try {
    const response=await fetch("/api/status"); const state=await response.json();
    if(connectionId!==state.connection_id || !state.connected) {
      if(armed || pressed.size || pending.some(value=>value!==0)) releaseAll();
      if(connectionId!==state.connection_id) screenSnapshot=0;
      connectionId=state.connection_id;
    }
    usbConnected=!!state.connected;
    $("enable").disabled=!usbConnected;
    document.querySelectorAll("[data-mask]").forEach(button=>{button.disabled=!usbConnected;});
    $("connection").textContent=state.connected ? (state.running ? "Meter connected · game running" : "Meter connected · waiting for game") : "Waiting for KM003C · reconnecting automatically…";
    $("dot").classList.toggle("connected",state.connected);
    $("pacing").disabled=pacingBusy || !state.connected || !state.pacing?.supported;
    if(!pacingBusy && state.pacing?.supported) $("pacing").value=state.pacing.smooth ? "1" : "0";
    if(!pacingBusy) $("pacing-error").textContent=state.pacing_error || "";
    lcd=state.display || null; paintLcd(state.connected);
    if(!displayBusy) $("display-error").textContent=state.display_error || "";
    $("performance").textContent=state.render_fps ? `${state.render_fps.toFixed(1)} game fps · ${state.frames} frames` : "—";
    $("error").textContent=state.error || "";
    if(state.screen && (state.screen.frame_id ?? state.screen.snapshot) !== screenSnapshot) {
      screenSnapshot=state.screen.frame_id ?? state.screen.snapshot;
      $("screen").src=`/api/frame.bmp?snapshot=${screenSnapshot}`;
      $("screen").hidden=false; $("screen-placeholder").hidden=true; $("save-frame").hidden=false;
      const fps=state.screen.preview_fps ? `${state.screen.preview_fps.toFixed(1)} fps` : "captured";
      $("screen-info").textContent=`${state.screen.width}×${state.screen.height} · ${fps} · frame ${state.screen.game_frame}`;
    }
    if(!state.screen) {
      $("screen").hidden=true; $("screen-placeholder").hidden=false; $("save-frame").hidden=true;
      $("screen-info").textContent="Waiting for capture…";
    }
    $("screen-error").textContent=state.screen_error || "";
    if(state.preview_enabled===false) $("screen-info").textContent="Preview paused";
  } catch { usbConnected=false; $("enable").disabled=true; $("connection").textContent="Controller offline"; $("dot").classList.remove("connected"); paintLcd(false); releaseAll(); }
}
fetch("/api/session").then(response=>response.json()).then(session=>{token=session.token;queue();void status();}).catch(error=>{$("error").textContent=error.message;});
setInterval(()=>{if(!stopped && armed) queue();},100);
setInterval(status,500);
paint();

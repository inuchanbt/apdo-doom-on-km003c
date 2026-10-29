"use strict";
const keyMap = {KeyW:2,ArrowUp:2,KeyS:128,ArrowDown:128,KeyA:4,ArrowLeft:4,
  KeyD:8,ArrowRight:8,Space:1,ControlLeft:1,ControlRight:1,KeyE:16,Escape:32,Enter:64,NumpadEnter:64};
const pressed = new Map();
let token = "", armed = false, stopped = false, sending = false;
let pending = [];
let screenSnapshot = 0;
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
      if (!response.ok) throw new Error("Controller disconnected. Release controls and restart the controller.");
    }
  } catch (error) { pending=[]; pressed.clear(); armed=false; paint(); if(!stopped) $("error").textContent=error.message; }
  finally { sending = false; }
}
function queue() { const value=mask(); if (pending.at(-1)!==value) pending.push(value); paint(); void sendPending(); }
function releaseAll() { pressed.clear(); armed=false; pending=[]; queue(); }
$("enable").addEventListener("click",()=>{armed=true;paint();});
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
  if (!(event.code in keyMap)) return;
  if (armed) event.preventDefault();
  pressed.delete(event.code); queue();
});
window.addEventListener("blur",releaseAll);
window.addEventListener("pagehide",releaseAll);
document.addEventListener("visibilitychange",()=>{if(document.hidden) releaseAll();});
document.querySelectorAll("[data-mask]").forEach(button=>{
  button.addEventListener("pointerdown",event=>{
    event.preventDefault(); armed=true; button.setPointerCapture(event.pointerId);
    pressed.set("pointer"+event.pointerId,Number(button.dataset.mask)); queue();
  });
  const release=event=>{pressed.delete("pointer"+event.pointerId);queue();};
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
    $("connection").textContent=state.connected ? (state.running ? "Meter connected · game running" : "Meter connected · waiting for game") : "Waiting for USB connection…";
    $("dot").classList.toggle("connected",state.connected);
    $("performance").textContent=state.render_fps ? `${state.render_fps.toFixed(1)} game fps · ${state.frames} frames` : "—";
    $("error").textContent=state.error || "";
    if(state.screen && state.screen.snapshot !== screenSnapshot) {
      screenSnapshot=state.screen.snapshot;
      $("screen").src=`/api/frame.bmp?snapshot=${screenSnapshot}`;
      $("screen").hidden=false; $("screen-placeholder").hidden=true; $("save-frame").hidden=false;
      const fps=state.screen.preview_fps ? `${state.screen.preview_fps.toFixed(1)} fps` : "captured";
      $("screen-info").textContent=`${state.screen.width}×${state.screen.height} · ${fps} · frame ${state.screen.game_frame}`;
    }
    $("screen-error").textContent=state.screen_error || "";
    if(state.preview_enabled===false) $("screen-info").textContent="Preview paused";
  } catch { $("connection").textContent="Controller offline"; $("dot").classList.remove("connected"); releaseAll(); }
}
fetch("/api/session").then(response=>response.json()).then(session=>{token=session.token;queue();void status();}).catch(error=>{$("error").textContent=error.message;});
setInterval(()=>{if(!stopped) queue();},100);
setInterval(status,500);
paint();

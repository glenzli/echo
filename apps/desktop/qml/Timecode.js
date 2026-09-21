.pragma library
// Millisecond time entry shared by source selection and arrangement navigation.
function parse(text) {
    const value = String(text).trim().replace(/：/g, ":").replace(",", ".");
    if (!/^\d+(?::\d{1,2}){0,2}(?:\.\d{1,3})?$/.test(value)) return null;
    const parts = value.split(":");
    if (parts.length > 1 && Number(parts[parts.length-1]) >= 60) return null;
    if (parts.length === 3 && Number(parts[1]) >= 60) return null;
    let seconds = 0;
    for (const part of parts) seconds = seconds * 60 + Number(part);
    const millis = Math.round(seconds * 1000);
    return Number.isSafeInteger(millis) && millis >= 0 && millis <= 2147483647 ? millis : null;
}
function format(millis) {
    const value = Math.max(0, Math.round(millis));
    const seconds = Math.floor(value / 1000);
    return Math.floor(seconds/3600) + ":" + String(Math.floor(seconds/60)%60).padStart(2,"0") + ":" + String(seconds%60).padStart(2,"0") + "." + String(value%1000).padStart(3,"0");
}

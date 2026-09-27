export type UserPosition = { latitude: number; longitude: number };
export function requestUserPosition(): Promise<UserPosition> {
 return new Promise((resolve,reject)=>{
  if (!navigator.geolocation) { reject(new Error("Geolocalización no disponible")); return; }
  navigator.geolocation.getCurrentPosition(
   ({coords}) => {
    if(!Number.isFinite(coords.latitude)||!Number.isFinite(coords.longitude)||Math.abs(coords.latitude)>90||Math.abs(coords.longitude)>180){reject(new Error("Coordenadas inválidas"));return;}
    resolve({latitude:coords.latitude,longitude:coords.longitude});
   },reject,{enableHighAccuracy:false,timeout:10000,maximumAge:60000}
  );
 });
}

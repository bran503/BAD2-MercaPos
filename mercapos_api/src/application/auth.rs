use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const LONGITUD_MINIMA_PASSWORD: usize = 8;
pub const LONGITUD_MAXIMA_PASSWORD: usize = 128;
const INTENTOS_LOGIN_MAXIMOS: u8 = 5;
const VENTANA_LOGIN: Duration = Duration::from_secs(15 * 60);

#[derive(Clone, Copy)]
struct IntentosLogin {
    fallos: u8,
    reinicio: Instant,
    bloqueado_hasta: Option<Instant>,
}

#[derive(Default)]
pub struct LimitadorLogin {
    intentos: Mutex<HashMap<String, IntentosLogin>>,
}

impl LimitadorLogin {
    pub fn permitido(&self, cuenta: &str) -> bool {
        let ahora = Instant::now();
        let mut intentos = self
            .intentos
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some(registro) = intentos.get(cuenta).copied() {
            if registro.bloqueado_hasta.is_some_and(|fin| ahora < fin) {
                return false;
            }
            if ahora >= registro.reinicio {
                intentos.remove(cuenta);
            }
        }
        true
    }

    pub fn registrar_fallo(&self, cuenta: &str) {
        let ahora = Instant::now();
        let mut intentos = self
            .intentos
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        intentos.retain(|_, registro| ahora < registro.reinicio);
        if !intentos.contains_key(cuenta)
            && intentos.len() >= 10_000
            && let Some(mas_antiguo) = intentos
                .iter()
                .min_by_key(|(_, registro)| registro.reinicio)
                .map(|(cuenta, _)| cuenta.clone())
        {
            intentos.remove(&mas_antiguo);
        }
        let registro = intentos.entry(cuenta.to_owned()).or_insert(IntentosLogin {
            fallos: 0,
            reinicio: ahora + VENTANA_LOGIN,
            bloqueado_hasta: None,
        });
        if ahora >= registro.reinicio {
            registro.fallos = 0;
            registro.reinicio = ahora + VENTANA_LOGIN;
            registro.bloqueado_hasta = None;
        }
        registro.fallos = registro.fallos.saturating_add(1);
        if registro.fallos >= INTENTOS_LOGIN_MAXIMOS {
            registro.bloqueado_hasta = Some(ahora + VENTANA_LOGIN);
        }
    }

    pub fn registrar_exito(&self, cuenta: &str) {
        self.intentos
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(cuenta);
    }
}

pub fn ahora_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duracion| duracion.as_secs() as i64)
        .unwrap_or_default()
}

pub fn validar_password(password: &str) -> Result<(), String> {
    if password.chars().count() < LONGITUD_MINIMA_PASSWORD {
        return Err(format!(
            "La contraseña debe tener al menos {LONGITUD_MINIMA_PASSWORD} caracteres"
        ));
    }
    if password.chars().count() > LONGITUD_MAXIMA_PASSWORD {
        return Err(format!(
            "La contraseña no puede superar {LONGITUD_MAXIMA_PASSWORD} caracteres"
        ));
    }
    Ok(())
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

pub fn verificar_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub async fn hash_password_async(password: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|error| format!("No se pudo calcular el hash: {error}"))?
}

pub async fn verificar_password_async(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || verificar_password(&password, &hash))
        .await
        .unwrap_or(false)
}

pub fn generar_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn nuevo_id(prefijo: &str) -> String {
    format!("{prefijo}:{}", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_longitud_de_password() {
        assert!(validar_password("corta").is_err());
        assert!(validar_password("correcta-1").is_ok());
        assert!(validar_password(&"x".repeat(LONGITUD_MAXIMA_PASSWORD + 1)).is_err());
    }

    #[test]
    fn hash_y_verificacion_de_password() {
        let hash = hash_password("secreto-123").unwrap();
        assert!(verificar_password("secreto-123", &hash));
        assert!(!verificar_password("secreto-124", &hash));
        assert!(!verificar_password("secreto-123", "hash-invalido"));
    }

    #[test]
    fn el_token_se_guarda_hasheado() {
        let token = generar_token();
        let hash = hash_token(&token);
        assert_ne!(token, hash);
        assert_eq!(hash.len(), 64);
        assert_eq!(hash_token(&token), hash);
    }

    #[test]
    fn limita_y_reinicia_intentos_de_login() {
        let limitador = LimitadorLogin::default();
        for _ in 0..INTENTOS_LOGIN_MAXIMOS {
            assert!(limitador.permitido("usuario:test"));
            limitador.registrar_fallo("usuario:test");
        }
        assert!(!limitador.permitido("usuario:test"));
        limitador.registrar_exito("usuario:test");
        assert!(limitador.permitido("usuario:test"));
    }
}

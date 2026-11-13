use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use backend::domain::entities::usuario::{Rol, Usuario};
use backend::domain::repositories::repository_error::RepositoryError;
use backend::domain::repositories::usuario_repository::UsuarioRepository;

use backend::application::use_cases::usuario::actualizar_usuario::ActualizarUsuario;
use backend::application::use_cases::usuario::crear_usuario::CrearUsuario;
use backend::application::use_cases::usuario::listar_usuarios_por_puesto::ListarUsuariosPorPuesto;
use backend::application::use_cases::usuario::obtener_usuario::ObtenerUsuario;
use backend::application::use_cases::usuario::usuario_error::UsuarioError;

// Repositorio falso
struct UsuarioRepositoryFake {
    usuarios: Mutex<HashMap<String, Usuario>>,
}

impl UsuarioRepositoryFake {
    fn new() -> Self {
        Self {
            usuarios: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl UsuarioRepository for UsuarioRepositoryFake {
    async fn crear(&self, usuario: Usuario) -> Result<(), RepositoryError> {
        let mut usuarios = self.usuarios.lock().unwrap();

        if usuarios.contains_key(&usuario.id) {
            return Err(RepositoryError::AlreadyExists);
        }

        usuarios.insert(usuario.id.clone(), usuario);
        Ok(())
    }

    async fn actualizar(&self, usuario: Usuario) -> Result<(), RepositoryError> {
        let mut usuarios = self.usuarios.lock().unwrap();

        if !usuarios.contains_key(&usuario.id) {
            return Err(RepositoryError::NotFound);
        }

        usuarios.insert(usuario.id.clone(), usuario);
        Ok(())
    }

    async fn obtener_por_id(&self, id: &str) -> Result<Option<Usuario>, RepositoryError> {
        let usuarios = self.usuarios.lock().unwrap();
        Ok(usuarios.get(id).cloned())
    }

    async fn listar_por_puesto(&self, puesto_id: &str) -> Result<Vec<Usuario>, RepositoryError> {
        let usuarios = self.usuarios.lock().unwrap();

        Ok(usuarios
            .values()
            .filter(|u| u.puesto_id == puesto_id)
            .cloned()
            .collect())
    }
}

fn usuario_prueba() -> Usuario {
    Usuario {
        id: "usuario_001".into(),
        nombre: "David".into(),
        rol: Rol::Administrador,
        puesto_id: "puesto_001".into(),
        activo: true,
        revision: None,
    }
}

// 1. Crear usuario válido
#[tokio::test]
async fn crear_usuario_valido() {
    let repositorio = Arc::new(UsuarioRepositoryFake::new());
    let caso_uso = CrearUsuario::new(repositorio.clone());

    let usuario = usuario_prueba();

    assert!(caso_uso.ejecutar(usuario.clone()).await.is_ok());

    let encontrado = repositorio.obtener_por_id(&usuario.id).await.unwrap();

    assert!(encontrado.is_some());
}

// 2. Rechazar nombre vacío
#[tokio::test]
async fn rechazar_nombre_vacio() {
    let repositorio = Arc::new(UsuarioRepositoryFake::new());
    let caso_uso = CrearUsuario::new(repositorio);

    let mut usuario = usuario_prueba();
    usuario.nombre = " ".into();

    let resultado = caso_uso.ejecutar(usuario).await;

    assert!(matches!(resultado, Err(UsuarioError::DatosInvalidos(_))));
}

// 3. Rechazar usuario duplicado
#[tokio::test]
async fn rechazar_usuario_duplicado() {
    let repositorio = Arc::new(UsuarioRepositoryFake::new());
    let caso_uso = CrearUsuario::new(repositorio);

    let usuario = usuario_prueba();

    caso_uso.ejecutar(usuario.clone()).await.unwrap();

    let resultado = caso_uso.ejecutar(usuario).await;

    assert!(matches!(
        resultado,
        Err(UsuarioError::Repositorio(RepositoryError::AlreadyExists))
    ));
}

// 4. Actualizar usuario inexistente
#[tokio::test]
async fn actualizar_usuario_inexistente() {
    let repositorio = Arc::new(UsuarioRepositoryFake::new());
    let caso_uso = ActualizarUsuario::new(repositorio);

    let resultado = caso_uso.ejecutar(usuario_prueba()).await;

    assert!(matches!(
        resultado,
        Err(UsuarioError::Repositorio(RepositoryError::NotFound))
    ));
}

// 5. Obtener usuario
#[tokio::test]
async fn obtener_usuario_existente() {
    let repositorio = Arc::new(UsuarioRepositoryFake::new());

    let usuario = usuario_prueba();
    repositorio.crear(usuario.clone()).await.unwrap();

    let caso_uso = ObtenerUsuario::new(repositorio);

    let resultado = caso_uso.ejecutar(&usuario.id).await.unwrap();

    assert_eq!(resultado, Some(usuario));
}

// 6. Listar usuarios por puesto
#[tokio::test]
async fn listar_usuarios_por_puesto() {
    let repositorio = Arc::new(UsuarioRepositoryFake::new());

    repositorio.crear(usuario_prueba()).await.unwrap();

    let caso_uso = ListarUsuariosPorPuesto::new(repositorio);

    let resultado = caso_uso.ejecutar("puesto_001").await.unwrap();

    assert_eq!(resultado.len(), 1);
    assert_eq!(resultado[0].nombre, "David");
}

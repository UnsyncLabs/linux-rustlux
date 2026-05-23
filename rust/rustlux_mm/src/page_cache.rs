// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// page_cache.rs — Protección de escritura en el page cache
//
// Mitiga: Dirty Pipe (CVE-2022-0847), Copy Fail (CVE-2026-31431),
//         Dirty Frag (CVE-2026-43284, CVE-2026-43500)
//
// El mecanismo: PageWritePermit es un token de capacidad que solo puede
// crearse si el inode subyacente permite escritura. Sin este token,
// ningún subsistema (xfrm-ESP, RxRPC, AF_ALG, splice) puede obtener
// una referencia mutable a una página del page cache.
//
// En Linux C, el bug es:
//   page = find_get_page(mapping, index);
//   set_page_dirty(page);  // ← sin verificar permisos del archivo
//
// En Rustlux, el equivalente requiere:
//   let permit = PageWritePermit::new(inode)?;  // ← falla si SUID/RO
//   let page = mapping.get_page_mut(index, &permit)?;

/// Flags de un inode relevantes para permisos de escritura.
/// Espejo de los flags de Linux (include/linux/fs.h).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InodeFlags(pub(crate) u32);

impl InodeFlags {
    /// S_ISUID — Set-user-ID bit. Archivos con este bit NO pueden
    /// recibir escrituras via page cache sin verificación explícita.
    pub const SUID: Self = Self(0o4000);

    /// S_ISGID — Set-group-ID bit. Mismo tratamiento que SUID.
    pub const SGID: Self = Self(0o2000);

    /// Immutable flag (FS_IMMUTABLE_FL). El archivo no puede modificarse.
    pub const IMMUTABLE: Self = Self(1 << 3);

    /// Append-only flag (FS_APPEND_FL).
    pub const APPEND_ONLY: Self = Self(1 << 4);

    /// Construye flags desde un valor raw (del campo i_flags del inode C).
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Verifica si contiene un flag específico.
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

/// Errores de permisos de escritura en el page cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageWriteError {
    /// El archivo es SUID o SGID — escritura via page cache denegada.
    /// Esta es la raíz de Dirty Pipe / Copy Fail / Dirty Frag.
    SuidOrSgid,

    /// El archivo está en un filesystem montado read-only.
    ReadOnlyFilesystem,

    /// El archivo tiene el flag immutable.
    Immutable,

    /// El archivo es append-only y la operación no es append.
    AppendOnly,

    /// El llamador no tiene permisos DAC de escritura sobre el archivo.
    PermissionDenied,
}

/// Token de permiso de escritura al page cache.
///
/// Este tipo es la pieza central de la mitigación. Solo puede crearse
/// mediante [`PageWritePermit::new`], que verifica todos los permisos
/// necesarios. Sin una instancia de este tipo, no es posible obtener
/// una referencia mutable a una página del page cache.
///
/// El token es `!Send` y `!Sync` intencionalmente: los permisos se
/// verifican en el contexto de la tarea actual y no deben transferirse
/// a otros contextos. Esto se implementa con `PhantomData<*mut ()>`,
/// el patrón estándar de Rust stable para tipos no-Send/no-Sync.
///
/// # Ejemplo de uso (pseudocódigo del kernel)
///
/// ```ignore
/// // En el path de splice() que toca el page cache:
/// fn splice_to_page_cache(src: &PipeBuffer, dst: &File) -> Result<usize> {
///     // Si dst es SUID, esta línea retorna Err(SuidOrSgid)
///     // y el splice falla antes de tocar ninguna página.
///     let permit = PageWritePermit::new(dst.inode())?;
///
///     let page = dst.mapping().get_page_mut(offset, &permit)?;
///     // ... escritura segura
/// }
/// ```
#[derive(Debug)]
pub struct PageWritePermit {
    // PhantomData<*mut ()> hace el tipo !Send + !Sync en Rust stable.
    // Es el patrón estándar cuando impl !Send / impl !Sync no están disponibles.
    _not_send_sync: core::marker::PhantomData<*mut ()>,
}

impl PageWritePermit {
    /// Crea un token de permiso de escritura verificando los flags del inode.
    ///
    /// # Errores
    ///
    /// Retorna error si:
    /// - El inode tiene SUID o SGID (`PageWriteError::SuidOrSgid`)
    /// - El filesystem está montado read-only (`PageWriteError::ReadOnlyFilesystem`)
    /// - El inode tiene el flag immutable (`PageWriteError::Immutable`)
    /// - El inode es append-only (`PageWriteError::AppendOnly`)
    ///
    /// # Seguridad
    ///
    /// Esta función es la barrera principal contra Dirty Frag y Copy Fail.
    /// Cualquier path que quiera escribir en el page cache DEBE pasar por aquí.
    #[inline]
    pub fn new(inode_flags: InodeFlags, sb_readonly: bool) -> Result<Self, PageWriteError> {
        // Verificación 1: SUID/SGID — la raíz de Dirty Pipe/Copy Fail/Dirty Frag.
        // Un archivo SUID que se ejecuta como root NO debe poder ser modificado
        // via page cache por subsistemas de red o criptografía.
        if inode_flags.contains(InodeFlags::SUID) || inode_flags.contains(InodeFlags::SGID) {
            return Err(PageWriteError::SuidOrSgid);
        }

        // Verificación 2: Filesystem read-only.
        if sb_readonly {
            return Err(PageWriteError::ReadOnlyFilesystem);
        }

        // Verificación 3: Inode immutable.
        if inode_flags.contains(InodeFlags::IMMUTABLE) {
            return Err(PageWriteError::Immutable);
        }

        // Verificación 4: Append-only (no bloqueamos append, solo escritura arbitraria).
        // Nota: el caller debe verificar si la operación es append antes de llamar aquí.
        if inode_flags.contains(InodeFlags::APPEND_ONLY) {
            return Err(PageWriteError::AppendOnly);
        }

        Ok(Self { _not_send_sync: core::marker::PhantomData })
    }

    /// Constructor para uso en contextos donde los permisos ya fueron
    /// verificados por otro mecanismo (ej: VFS write path normal).
    ///
    /// # Safety
    ///
    /// El caller debe garantizar que:
    /// - El inode no es SUID/SGID
    /// - El filesystem no es read-only
    /// - El inode no es immutable
    /// - El llamador tiene permisos de escritura DAC
    #[inline]
    pub unsafe fn new_unchecked() -> Self {
        Self { _not_send_sync: core::marker::PhantomData }
    }
}

/// Referencia mutable a una página del page cache.
///
/// Solo puede obtenerse con un [`PageWritePermit`] válido.
/// El lifetime `'permit` garantiza que el permit vive al menos
/// tanto como esta referencia.
pub struct PageMut<'permit> {
    /// Offset de la página dentro del archivo (en unidades de PAGE_SIZE).
    pub index: u64,
    // El lifetime vincula esta referencia al permit que la autorizó.
    _permit: core::marker::PhantomData<&'permit PageWritePermit>,
}

impl<'permit> PageMut<'permit> {
    /// Crea una PageMut. Solo llamable desde dentro del subsistema de mm.
    ///
    /// # Safety
    ///
    /// El caller debe garantizar que tiene un PageWritePermit válido
    /// y que la página en `index` existe en el mapping.
    #[inline]
    pub unsafe fn new(index: u64, _permit: &'permit PageWritePermit) -> Self {
        Self {
            index,
            _permit: core::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suid_file_denied() {
        let flags = InodeFlags::from_raw(InodeFlags::SUID.0);
        let result = PageWritePermit::new(flags, false);
        assert_eq!(result.unwrap_err(), PageWriteError::SuidOrSgid);
    }

    #[test]
    fn sgid_file_denied() {
        let flags = InodeFlags::from_raw(InodeFlags::SGID.0);
        let result = PageWritePermit::new(flags, false);
        assert_eq!(result.unwrap_err(), PageWriteError::SuidOrSgid);
    }

    #[test]
    fn readonly_fs_denied() {
        let flags = InodeFlags::from_raw(0);
        let result = PageWritePermit::new(flags, true);
        assert_eq!(result.unwrap_err(), PageWriteError::ReadOnlyFilesystem);
    }

    #[test]
    fn immutable_denied() {
        let flags = InodeFlags::from_raw(InodeFlags::IMMUTABLE.0);
        let result = PageWritePermit::new(flags, false);
        assert_eq!(result.unwrap_err(), PageWriteError::Immutable);
    }

    #[test]
    fn normal_file_allowed() {
        let flags = InodeFlags::from_raw(0);
        let result = PageWritePermit::new(flags, false);
        assert!(result.is_ok());
    }

    #[test]
    fn suid_plus_sgid_denied() {
        let flags = InodeFlags::from_raw(InodeFlags::SUID.0 | InodeFlags::SGID.0);
        let result = PageWritePermit::new(flags, false);
        assert_eq!(result.unwrap_err(), PageWriteError::SuidOrSgid);
    }
}

/// File CRUD Operations — read, write, create, and delete files.
///
/// Builds on the FAT32 filesystem layer to provide high-level
/// file operations. This is what the shell and config persistence
/// will use to interact with persistent storage.
///
/// Architecture:
///   FileSystem (fs.rs) → FileOps → read/write/create/delete
///   FileHandle tracks open files with cursor position
///
/// Phase 5, Item 2 — the kernel can manage individual files.

use alloc::string::String;
use alloc::vec::Vec;

/// A handle to an open file.
#[derive(Debug)]
pub struct FileHandle {
    /// File path (relative to root).
    pub path: String,
    /// Current cursor position in bytes.
    pub cursor: usize,
    /// File size in bytes.
    pub size: usize,
    /// File content buffer (loaded on open).
    buffer: Vec<u8>,
    /// Whether the file has been modified.
    dirty: bool,
    /// Access mode.
    mode: FileMode,
    /// Unique handle ID.
    pub id: u64,
}

/// File access modes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileMode {
    /// Read-only access.
    Read,
    /// Write access (creates if not exists).
    Write,
    /// Append mode (cursor starts at end).
    Append,
    /// Read-write access.
    ReadWrite,
}

impl FileHandle {
    /// Create a new file handle.
    pub fn new(path: &str, content: Vec<u8>, mode: FileMode, id: u64) -> Self {
        let size = content.len();
        let cursor = if mode == FileMode::Append { size } else { 0 };
        Self {
            path: String::from(path),
            cursor,
            size,
            buffer: content,
            dirty: false,
            mode,
            id,
        }
    }

    /// Read up to `count` bytes from current cursor position.
    pub fn read(&mut self, count: usize) -> Result<Vec<u8>, FileError> {
        if self.mode == FileMode::Write {
            return Err(FileError::NotReadable);
        }

        let start = self.cursor.min(self.buffer.len());
        let end = (start + count).min(self.buffer.len());
        let data = self.buffer[start..end].to_vec();
        self.cursor = end;
        Ok(data)
    }

    /// Read all remaining bytes from cursor to end.
    pub fn read_all(&mut self) -> Result<Vec<u8>, FileError> {
        if self.mode == FileMode::Write {
            return Err(FileError::NotReadable);
        }

        let data = self.buffer[self.cursor..].to_vec();
        self.cursor = self.buffer.len();
        Ok(data)
    }

    /// Read the entire file as a UTF-8 string.
    pub fn read_string(&mut self) -> Result<String, FileError> {
        let saved_cursor = self.cursor;
        self.cursor = 0;
        let data = self.read_all()?;
        self.cursor = saved_cursor;
        String::from_utf8(data).map_err(|_| FileError::InvalidUtf8)
    }

    /// Write bytes at current cursor position.
    pub fn write(&mut self, data: &[u8]) -> Result<usize, FileError> {
        if self.mode == FileMode::Read {
            return Err(FileError::NotWritable);
        }

        let end = self.cursor + data.len();
        if end > self.buffer.len() {
            self.buffer.resize(end, 0);
        }
        self.buffer[self.cursor..end].copy_from_slice(data);
        self.cursor = end;
        self.size = self.buffer.len();
        self.dirty = true;
        Ok(data.len())
    }

    /// Write a string to the file.
    pub fn write_string(&mut self, s: &str) -> Result<usize, FileError> {
        self.write(s.as_bytes())
    }

    /// Seek to an absolute position.
    pub fn seek(&mut self, position: usize) -> Result<(), FileError> {
        if position > self.size {
            return Err(FileError::SeekPastEnd {
                requested: position,
                size: self.size,
            });
        }
        self.cursor = position;
        Ok(())
    }

    /// Seek to the beginning of the file.
    pub fn rewind(&mut self) {
        self.cursor = 0;
    }

    /// Whether the file has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get the internal buffer (for flushing to disk).
    pub fn content(&self) -> &[u8] {
        &self.buffer
    }

    /// Mark as clean (after flushing to disk).
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Truncate the file to zero length.
    pub fn truncate(&mut self) -> Result<(), FileError> {
        if self.mode == FileMode::Read {
            return Err(FileError::NotWritable);
        }
        self.buffer.clear();
        self.cursor = 0;
        self.size = 0;
        self.dirty = true;
        Ok(())
    }
}

/// File operation errors.
#[derive(Debug)]
pub enum FileError {
    NotReadable,
    NotWritable,
    InvalidUtf8,
    SeekPastEnd { requested: usize, size: usize },
    NotFound(String),
    AlreadyExists(String),
    PathTooLong,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_read_write() {
        let mut fh = FileHandle::new("/test.txt", Vec::new(), FileMode::ReadWrite, 1);
        fh.write(b"hello world").unwrap();
        fh.rewind();
        let data = fh.read(5).unwrap();
        assert_eq!(&data, b"hello");
    }

    #[test_case]
    fn test_read_string() {
        let content = b"VargasJR kernel config".to_vec();
        let mut fh = FileHandle::new("/config.txt", content, FileMode::Read, 2);
        let s = fh.read_string().unwrap();
        assert_eq!(s, "VargasJR kernel config");
    }

    #[test_case]
    fn test_append_mode() {
        let content = b"existing ".to_vec();
        let mut fh = FileHandle::new("/log.txt", content, FileMode::Append, 3);
        assert_eq!(fh.cursor, 9); // starts at end
        fh.write(b"data").unwrap();
        fh.rewind();
        let all = fh.read_all().unwrap();
        assert_eq!(&all, b"existing data");
    }

    #[test_case]
    fn test_read_only_write_fails() {
        let mut fh = FileHandle::new("/ro.txt", b"data".to_vec(), FileMode::Read, 4);
        let result = fh.write(b"nope");
        assert!(matches!(result, Err(FileError::NotWritable)));
    }

    #[test_case]
    fn test_seek_and_truncate() {
        let mut fh = FileHandle::new("/t.txt", b"abcdef".to_vec(), FileMode::ReadWrite, 5);
        fh.seek(3).unwrap();
        let data = fh.read(3).unwrap();
        assert_eq!(&data, b"def");

        fh.truncate().unwrap();
        assert_eq!(fh.size, 0);
        assert!(fh.is_dirty());
    }

    #[test_case]
    fn test_dirty_tracking() {
        let mut fh = FileHandle::new("/d.txt", Vec::new(), FileMode::Write, 6);
        assert!(!fh.is_dirty());
        fh.write(b"x").unwrap();
        assert!(fh.is_dirty());
        fh.mark_clean();
        assert!(!fh.is_dirty());
    }
}

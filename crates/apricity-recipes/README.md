# apricity-recipes

Recipe model and in-memory store. Persistence is delegated to the host shell
via a trait — this crate has no file I/O.

## Owns

- `Recipe` struct (wraps `GradeParameters` + metadata)
- In-memory recipe store
- `RecipeStore` persistence trait

## Does not own

- File or `localStorage` I/O (host shell)
- Recipe sharing transport (out of scope until v1.5+)

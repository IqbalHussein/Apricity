import { createMemo, createSignal, For, Show } from "solid-js";
import type { JSX } from "solid-js";
import type { Recipe } from "../core/wasm";
import { requireFeature } from "../entitlements/store";
import { pickFile } from "../lib/download";
import {
  applyRecipe,
  defaultRecipeName,
  deleteRecipe,
  exportRecipeFile,
  importRecipeFile,
  recipeError,
  recipeLibraryOpen,
  recipeLimitLabel,
  recipeThumbnail,
  renameRecipe,
  saveRecipe,
  saveRecipeOpen,
  savedRecipes,
  setRecipeLibraryOpen,
  setSaveRecipeOpen,
  starterRecipes,
} from "../state/recipes";
import { currentSource } from "../state/project";
import { settledFrameCount } from "../video/player";
import { Dialog } from "./Dialog";
import { DownloadIcon, PencilIcon, TrashIcon } from "./Icons";
import { UpgradeNotice } from "./UpgradeNotice";

/**
 * Open the Save recipe dialog, unless the Free limit is reached (then a notice
 * explains it).
 */
export function openSaveRecipe(): void {
  if (!currentSource()) return;
  if (requireFeature("recipe_save", { recipeCount: savedRecipes().length })) setSaveRecipeOpen(true);
}

/** Dialog that asks for a name and saves the current grade as a recipe. */
export function SaveRecipeDialog(): JSX.Element {
  const [name, setName] = createSignal(defaultRecipeName());
  const [problem, setProblem] = createSignal<string | null>(null);
  const submit = (e: SubmitEvent, close: () => void): void => {
    e.preventDefault();
    if (saveRecipe(name())) close();
    else setProblem(recipeError());
  };
  return (
    <Dialog title="Save recipe" testId="save-recipe-dialog" onClose={() => setSaveRecipeOpen(false)}>
      {(() => {
        let form!: HTMLFormElement;
        const close = (): void => form.closest("dialog")?.close();
        return (
          <form ref={form} class="dialog-form" onSubmit={(e) => submit(e, close)}>
            <p class="dialog-note">
              Saves Correction and Grade together, so you can apply this look to any image.
            </p>
            <label class="field">
              <span class="field-label">Recipe name</span>
              <input
                class="text-input"
                type="text"
                data-testid="recipe-name-input"
                maxlength="64"
                autocomplete="off"
                autofocus
                value={name()}
                onInput={(e) => setName(e.currentTarget.value)}
              />
            </label>
            <Show when={problem()}>
              <p class="error-text" role="alert">
                {problem()}
              </p>
            </Show>
            <div class="dialog-actions">
              <button type="button" class="btn" onClick={close}>
                Cancel
              </button>
              <button type="submit" class="btn btn-primary" data-testid="save-recipe-confirm">
                Save recipe
              </button>
            </div>
          </form>
        );
      })()}
    </Dialog>
  );
}

function Thumb(props: { recipe: Recipe }): JSX.Element {
  const url = createMemo(() => {
    currentSource();
    settledFrameCount();
    return recipeThumbnail(props.recipe);
  });
  return (
    <Show when={url()} fallback={<span class="thumb thumb-empty" aria-hidden="true" />}>
      {(u) => <img class="thumb" src={u()} alt="" draggable={false} />}
    </Show>
  );
}

function SavedCard(props: { recipe: Recipe; onApplied: () => void }): JSX.Element {
  const [renaming, setRenaming] = createSignal(false);
  const [confirming, setConfirming] = createSignal(false);
  const [draft, setDraft] = createSignal("");
  const r = (): Recipe => props.recipe;
  const commit = (): void => {
    if (!renaming()) return;
    setRenaming(false);
    if (draft().trim() !== "" && draft().trim() !== r().name) renameRecipe(r(), draft());
  };
  return (
    <article class="recipe-card" data-testid="recipe-card" data-recipe-id={r().id}>
      <button
        type="button"
        class="recipe-apply"
        data-testid="recipe-apply"
        disabled={!currentSource()}
        aria-label={`Apply recipe: ${r().name}`}
        onClick={() => {
          applyRecipe(r());
          props.onApplied();
        }}
      >
        <Thumb recipe={r()} />
      </button>
      <div class="recipe-meta">
        <Show
          when={renaming()}
          fallback={
            <span class="recipe-name" data-testid="recipe-name" title={r().name}>
              {r().name}
            </span>
          }
        >
          <input
            class="text-input recipe-rename"
            data-testid="recipe-rename-input"
            type="text"
            maxlength="64"
            aria-label="New recipe name"
            value={draft()}
            ref={(el) => queueMicrotask(() => (el.focus(), el.select()))}
            onInput={(e) => setDraft(e.currentTarget.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              if (e.key === "Enter") commit();
              else if (e.key === "Escape") setRenaming(false);
            }}
          />
        </Show>
        <Show
          when={!confirming()}
          fallback={
            <span class="recipe-confirm" role="group" aria-label={`Delete ${r().name}?`}>
              <button
                type="button"
                class="btn btn-small btn-danger"
                data-testid="recipe-delete-confirm"
                onClick={() => deleteRecipe(r().id)}
              >
                Delete
              </button>
              <button type="button" class="btn btn-small" onClick={() => setConfirming(false)}>
                Keep
              </button>
            </span>
          }
        >
          <span class="recipe-tools">
            <button
              type="button"
              class="icon-btn"
              data-testid="recipe-rename"
              aria-label={`Rename ${r().name}`}
              title="Rename"
              onClick={() => {
                setDraft(r().name);
                setRenaming(true);
              }}
            >
              <PencilIcon size={14} />
            </button>
            <button
              type="button"
              class="icon-btn"
              data-testid="recipe-export"
              aria-label={`Export ${r().name} as .json`}
              title="Export as .json"
              onClick={() => exportRecipeFile(r())}
            >
              <DownloadIcon size={14} />
            </button>
            <button
              type="button"
              class="icon-btn"
              data-testid="recipe-delete"
              aria-label={`Delete ${r().name}`}
              title="Delete"
              onClick={() => setConfirming(true)}
            >
              <TrashIcon size={14} />
            </button>
          </span>
        </Show>
      </div>
    </article>
  );
}

/** The Recipe Library dialog (PRD §6.5): starters, saved recipes, import and export. */
export function RecipeLibrary(): JSX.Element {
  const close = (): void => {
    setRecipeLibraryOpen(false);
  };
  const starters = starterRecipes();
  return (
    <Dialog title="Recipes" wide testId="recipe-library" onClose={close}>
      <div class="lib-toolbar">
        <button
          type="button"
          class="btn"
          data-testid="library-save"
          disabled={!currentSource()}
          onClick={() => openSaveRecipe()}
        >
          Save recipe
        </button>
        <button
          type="button"
          class="btn"
          data-testid="recipe-import"
          onClick={() => pickFile("application/json,.json", (f) => void importRecipeFile(f))}
        >
          Import recipe
        </button>
        <Show when={recipeLimitLabel()}>
          <span class="lib-count" data-testid="recipe-count">
            {recipeLimitLabel()}
          </span>
        </Show>
      </div>
      <Show when={recipeError()}>
        <p class="error-text" role="alert">
          {recipeError()}
        </p>
      </Show>
      <h3 class="lib-heading">Your recipes</h3>
      <Show
        when={savedRecipes().length > 0}
        fallback={
          <p class="dialog-note" data-testid="recipe-empty">
            No recipes yet. Save the current grade to keep it here.
          </p>
        }
      >
        <div class="recipe-grid">
          <For each={savedRecipes()}>{(r) => <SavedCard recipe={r} onApplied={close} />}</For>
        </div>
      </Show>
      <h3 class="lib-heading">Starter looks</h3>
      <div class="recipe-grid">
        <For each={starters}>
          {(r) => (
            <article class="recipe-card" data-testid="starter-card">
              <button
                type="button"
                class="recipe-apply"
                disabled={!currentSource()}
                aria-label={`Apply recipe: ${r.name}`}
                onClick={() => {
                  applyRecipe(r);
                  close();
                }}
              >
                <Thumb recipe={r} />
              </button>
              <div class="recipe-meta">
                <span class="recipe-name">{r.name}</span>
              </div>
            </article>
          )}
        </For>
      </div>
      <UpgradeNotice feature="recipe_save" />
    </Dialog>
  );
}

/** Quick-mode strip of single-tap looks: starters first, then saved recipes (PRD §8.2). */
export function RecipeStrip(): JSX.Element {
  const starters = starterRecipes();
  const items = createMemo(() => [...savedRecipes(), ...starters]);
  return (
    <Show when={currentSource()}>
      <div class="recipe-strip" data-testid="recipe-strip" role="group" aria-label="Recipes">
        <ul class="strip-list">
          <For each={items()}>
            {(r) => (
              <li>
                <button
                  type="button"
                  class="strip-item"
                  data-testid="recipe-chip"
                  aria-label={`Apply recipe: ${r.name}`}
                  onClick={() => applyRecipe(r)}
                >
                  <Thumb recipe={r} />
                  <span class="strip-name">{r.name}</span>
                </button>
              </li>
            )}
          </For>
        </ul>
        <button
          type="button"
          class="btn btn-quiet strip-more"
          data-testid="recipe-strip-more"
          onClick={() => setRecipeLibraryOpen(true)}
        >
          All recipes
        </button>
      </div>
    </Show>
  );
}

/** Mounts the recipe dialogs when they are open. Place once in the shell. */
export function RecipeDialogs(): JSX.Element {
  return (
    <>
      <Show when={recipeLibraryOpen()}>
        <RecipeLibrary />
      </Show>
      <Show when={saveRecipeOpen()}>
        <SaveRecipeDialog />
      </Show>
    </>
  );
}

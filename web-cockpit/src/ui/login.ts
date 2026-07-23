export interface LoginViewOptions {
  fetch: typeof globalThis.fetch;
  loginUrl?: string;
}

/** Renders the pre-protocol operator login and resolves after authentication. */
export function waitForOperatorLogin(
  document: Document,
  mount: HTMLElement,
  options: LoginViewOptions,
): Promise<void> {
  const section = document.createElement("section");
  section.className = "login-view";
  section.setAttribute("aria-labelledby", "patchbay-login-title");

  const card = document.createElement("div");
  card.className = "card card--raised login-card";
  const title = document.createElement("h1");
  title.id = "patchbay-login-title";
  title.className = "login-card__title";
  title.textContent = "Sign in to Patchbay";
  const description = document.createElement("p");
  description.className = "login-card__description";
  description.textContent = "Authenticate as the operator to open the cockpit.";

  const form = document.createElement("form");
  form.className = "login-form";
  const actorField = field(document, "Operator actor id", "text", "username");
  actorField.input.name = "actorId";
  actorField.input.autocomplete = "username";
  const passwordField = field(document, "Password", "password", "current-password");
  passwordField.input.name = "password";
  passwordField.input.autocomplete = "current-password";

  const error = document.createElement("div");
  error.className = "alert alert--danger login-form__error";
  error.setAttribute("role", "alert");
  error.hidden = true;

  const submit = document.createElement("button");
  submit.className = "btn btn-primary btn--lg";
  submit.type = "submit";
  submit.textContent = "Sign in";
  form.append(actorField.element, passwordField.element, error, submit);
  card.append(title, description, form);
  section.append(card);
  mount.replaceChildren(section);
  actorField.input.focus();

  return new Promise<void>((resolve) => {
    form.addEventListener("submit", (event) => {
      event.preventDefault();
      void (async () => {
        error.hidden = true;
        submit.disabled = true;
        try {
          const response = await options.fetch(options.loginUrl ?? "/login", {
            method: "POST",
            credentials: "same-origin",
            headers: {
              accept: "application/json",
              "content-type": "application/json",
            },
            body: JSON.stringify({
              actorId: actorField.input.value,
              password: passwordField.input.value,
            }),
          });
          if (!response.ok) {
            throw new Error(await loginFailureMessage(response));
          }
          resolve();
        } catch (cause) {
          error.textContent = cause instanceof Error ? cause.message : String(cause);
          error.hidden = false;
          passwordField.input.focus();
        } finally {
          submit.disabled = false;
        }
      })();
    });
  });
}

function field(
  document: Document,
  labelText: string,
  type: "text" | "password",
  autocomplete: "username" | "current-password",
): { element: HTMLLabelElement; input: HTMLInputElement } {
  const element = document.createElement("label");
  element.className = "field";
  const label = document.createElement("span");
  label.className = "field__label";
  label.textContent = labelText;
  const input = document.createElement("input");
  input.className = "input";
  input.type = type;
  input.autocomplete = autocomplete;
  input.required = true;
  element.append(label, input);
  return { element, input };
}

async function loginFailureMessage(response: Response): Promise<string> {
  try {
    const body: unknown = await response.json();
    if (
      typeof body === "object" &&
      body !== null &&
      "error" in body &&
      typeof body.error === "string"
    ) {
      return `Login failed: ${body.error}`;
    }
  } catch {
    // Fall back to the status when the response is not JSON.
  }
  return `Login failed (${response.status})`;
}

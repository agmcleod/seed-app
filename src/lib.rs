// (Lines like the one below ignore selected Clippy rules
//  - it's useful when you want to check your code with `cargo make verify`
// but some rules are too "annoying" or are not applicable for your case.)
#![allow(clippy::wildcard_imports)]
// TODO: Remove
#![allow(dead_code, unused_variables)]

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::mem;

use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use ulid::Ulid;
use web_sys;

const ENTER_KEY: &str = "Enter";
const ESC_KEY: &str = "Escape";
const STORAGE_KEY: &str = "todos-seed";

const ACTIVE: &str = "active";
const COMPLETED: &str = "completed";

// ------ ------
//     Init
// ------ ------

// `init` describes what should happen when your app started.
fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.subscribe(Msg::UrlChanged);

    Model {
        todos: LocalStorage::get(STORAGE_KEY).unwrap_or_default(),
        new_todo_title: "".to_string(),
        selected_todo: None,
        filter: Filter::from(url),
        base_url: Url::new(),
    }
}

// ------ ------
//     Model
// ------ ------

// `Model` describes our app state.
struct Model {
    todos: BTreeMap<Ulid, Todo>,
    new_todo_title: String,
    selected_todo: Option<SelectedTodo>,
    filter: Filter,
    base_url: Url,
}

impl Model {}

#[derive(Deserialize, Serialize)]
struct Todo {
    id: Ulid,
    title: String,
    completed: bool,
}

struct SelectedTodo {
    id: Ulid,
    title: String,
    input_element: ElRef<web_sys::HtmlInputElement>,
}

#[derive(Copy, Clone, Eq, PartialEq, EnumIter)]
enum Filter {
    All,
    Active,
    Completed,
}

impl From<Url> for Filter {
    fn from(mut url: Url) -> Self {
        match url.remaining_hash_path_parts().as_slice() {
            [ACTIVE] => Self::Active,
            [COMPLETED] => Self::Completed,
            _ => Self::All,
        }
    }
}

// ------ ------
//    Update
// ------ ------

// `Msg` describes the different events you can modify state with.
enum Msg {
    NewTodoTitleChanged(String),
    UrlChanged(subs::UrlChanged),
    // Basic todo operations
    CreateTodo,
    ToggleTodo(Ulid),
    RemoveTodo(Ulid),
    // Bulk todo operations
    CheckOrUncheckAll,
    ClearCompleted,
    // select operations
    SelectTodo(Option<Ulid>),
    SelectedTodoTitleChanged(String),
    SaveSelectedTodo,
}

// `update` describes how to handle each `Msg`.
fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::NewTodoTitleChanged(title) => {
            model.new_todo_title = title;
        }
        Msg::UrlChanged(subs::UrlChanged(url)) => {
            model.filter = Filter::from(url);
        }
        Msg::CreateTodo => {
            let title = model.new_todo_title.trim();
            if !title.is_empty() {
                let id = Ulid::new();
                model.todos.insert(
                    id,
                    Todo {
                        id,
                        title: title.to_owned(),
                        completed: false,
                    },
                );
                model.new_todo_title.clear();
            }
        }
        Msg::ToggleTodo(id) => {
            if let Some(todo) = model.todos.get_mut(&id) {
                todo.completed = not(todo.completed);
            }
        }
        Msg::RemoveTodo(id) => {
            model.todos.remove(&id);
        }
        Msg::CheckOrUncheckAll => {
            let all_checked = model.todos.values().all(|todo| todo.completed);
            for todo in model.todos.values_mut() {
                todo.completed = !all_checked;
            }
        }
        Msg::ClearCompleted => {
            model.todos = mem::take(&mut model.todos)
                .into_iter()
                .filter(|(_, todo)| !todo.completed)
                .collect();
        }
        Msg::SelectTodo(Some(id)) => {
            if let Some(todo) = model.todos.get(&id) {
                let input_element = ElRef::new();

                model.selected_todo = Some(SelectedTodo {
                    id,
                    title: todo.title.clone(),
                    input_element: input_element.clone(),
                });

                let title_length = u32::try_from(todo.title.len()).expect("title length as u32");
                orders.after_next_render(move |_| {
                    let input_element = input_element.get().expect("input element");

                    input_element
                        .set_selection_range(title_length, title_length)
                        .expect("move curse to end of input_element");
                });
            }
        }
        Msg::SelectTodo(None) => {
            model.selected_todo = None;
        }
        Msg::SelectedTodoTitleChanged(title) => {
            if let Some(selected_todo) = &mut model.selected_todo {
                selected_todo.title = title;
            }
        }
        Msg::SaveSelectedTodo => {
            if let Some(selected_todo) = model.selected_todo.take() {
                let title = selected_todo.title.trim();
                if title.is_empty() {
                    model.todos.remove(&selected_todo.id);
                } else {
                    if let Some(todo) = model.todos.get_mut(&selected_todo.id) {
                        todo.title = title.to_owned();
                    }
                }
            }
        }
    }

    LocalStorage::insert(STORAGE_KEY, &model.todos).expect("Save todos into local storage");
}

// ------ ------
//     View
// ------ ------

// `view` describes what to display.
fn view(model: &Model) -> Vec<Node<Msg>> {
    nodes![
        view_header(&model.new_todo_title),
        IF!(not(model.todos.is_empty()) => vec![
            view_main(&model.todos, model.selected_todo.as_ref(), model.filter),
            view_footer(&model.todos, model.filter),
        ]),
    ]
}

fn view_header(new_todo_title: &str) -> Node<Msg> {
    header![
        C!["header"],
        h1!["todos"],
        input![
            C!["new-todo"],
            attrs! {At::Placeholder => "What needs to be done?", At::AutoFocus => AtValue::None, At::Value => new_todo_title},
            input_ev(Ev::Input, Msg::NewTodoTitleChanged),
            keyboard_ev(Ev::KeyDown, |keyboard_event| {
                IF!(keyboard_event.key() == ENTER_KEY => Msg::CreateTodo)
            })
        ]
    ]
}

fn view_main(
    todos: &BTreeMap<Ulid, Todo>,
    selected_todo: Option<&SelectedTodo>,
    filter: Filter,
) -> Node<Msg> {
    section![
        C!["main"],
        view_toggle_all(todos),
        view_todo_list(todos, selected_todo, filter),
    ]
}

fn view_toggle_all(todos: &BTreeMap<Ulid, Todo>) -> Vec<Node<Msg>> {
    let all_completed = todos.values().all(|todo| todo.completed);
    vec![
        input![
            C!["toggle-all"],
            attrs! {At::Id => "toggle-all", At::Type => "checkbox", At::Checked => all_completed.as_at_value()},
            ev(Ev::Change, |_| Msg::CheckOrUncheckAll)
        ],
        label![attrs! {At::For => "toggle-all"}, "Mark all as complete"],
    ]
}

fn view_todo_list(
    todos: &BTreeMap<Ulid, Todo>,
    selected_todo: Option<&SelectedTodo>,
    filter: Filter,
) -> Node<Msg> {
    let todos = todos.values().filter(|todo| match filter {
        Filter::All => true,
        Filter::Active => !todo.completed,
        Filter::Completed => todo.completed,
    });

    ul![
        C!["todo-list"],
        todos.map(|todo| {
            let id = todo.id;
            let is_selected = Some(id) == selected_todo.map(|selected_todo| selected_todo.id);
            li![
                C![
                    IF!(todo.completed => "completed"),
                    IF!(is_selected => "editing")
                ],
                el_key(&todo.id),
                div![
                    C!["view"],
                    input![
                        C!["toggle"],
                        attrs! {At::Type => "checkbox", At::Checked => todo.completed.as_at_value()},
                        ev(Ev::Change, move |_| Msg::ToggleTodo(id))
                    ],
                    label![
                        &todo.title,
                        ev(Ev::DblClick, move |_| Msg::SelectTodo(Some(id)))
                    ],
                    button![C!["destroy"], ev(Ev::Click, move |_| Msg::RemoveTodo(id))],
                ],
                IF!(is_selected => {
                    let selected_todo = selected_todo.unwrap();
                    input![
                        C!["edit"],
                        el_ref(&selected_todo.input_element),
                        attrs! {At::Value => selected_todo.title},
                        keyboard_ev(Ev::KeyDown, |keyboard_event| {
                            IF!(keyboard_event.key() == ESC_KEY => Msg::SelectTodo(None))
                        }),
                        input_ev(Ev::Input, Msg::SelectedTodoTitleChanged),
                        keyboard_ev(Ev::KeyDown, |keyboard_event| {
                            match keyboard_event.key().as_str() {
                                ESC_KEY => Some(Msg::SelectTodo(None)),
                                ENTER_KEY => Some(Msg::SaveSelectedTodo),
                                _ => return None,
                            }
                        }),
                        ev(Ev::Blur, |_| Msg::SaveSelectedTodo),
                    ]
                })
            ]
        })
    ]
}

// ------ footer ------

fn view_footer(todos: &BTreeMap<Ulid, Todo>, selected_filter: Filter) -> Node<Msg> {
    let completed_count = todos.values().filter(|todo| todo.completed).count();
    let active_count = todos.len() - completed_count;

    footer![
        C!["footer"],
        // This should be `0 items left` by default
        span![
            C!["todo-count"],
            strong![completed_count],
            format!(" item{} left", if active_count == 1 { "" } else { "s" }),
        ],
        view_filters(selected_filter),
        IF!(completed_count > 0 => button![C!["clear-completed"], "Clear completed", ev(Ev::Click, |_| Msg::ClearCompleted)])
    ]
}

fn view_filters(selected_filter: Filter) -> Node<Msg> {
    ul![
        C!["filters"],
        Filter::iter().map(|filter| {
            let (link, title) = match filter {
                Filter::All => ("", "All"),
                Filter::Active => (ACTIVE, "Active"),
                Filter::Completed => (COMPLETED, "Completed"),
            };
            li![a![
                C![IF!(filter == selected_filter => "selected")],
                attrs! { At::Href => format!("#/{}", link) },
                title
            ]]
        })
    ]
}

// ------ ------
//     Start
// ------ ------

// (This function is invoked by `init` function in `index.html`.)
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    let root_element = document()
        .get_elements_by_class_name("todoapp")
        .item(0)
        .expect("Could not find .todoapp");

    App::start(root_element, init, update, view);
}

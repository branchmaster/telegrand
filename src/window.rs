use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::glib;
use std::sync::mpsc;

use crate::add_account_window::AddAccountWindow;
use crate::chat_box::ChatBox;
use crate::dialog_data::DialogData;
use crate::dialog_model::DialogModel;
use crate::dialog_row::DialogRow;
use crate::telegram;

mod imp {
    use super::*;
    use adw::subclass::prelude::*;
    use glib::subclass;
    use gtk::CompositeTemplate;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/com/github/melix99/telegrand/window.ui")]
    pub struct TelegrandWindow {
        #[template_child]
        pub add_account_window: TemplateChild<AddAccountWindow>,
        #[template_child]
        pub dialogs_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub chat_stack: TemplateChild<gtk::Stack>,
        pub dialog_model: DialogModel,
    }

    impl ObjectSubclass for TelegrandWindow {
        const NAME: &'static str = "TelegrandWindow";
        type Type = super::TelegrandWindow;
        type ParentType = adw::ApplicationWindow;
        type Interfaces = ();
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn new() -> Self {
            Self {
                add_account_window: TemplateChild::default(),
                dialogs_list: TemplateChild::default(),
                chat_stack: TemplateChild::default(),
                dialog_model: DialogModel::new(),
            }
        }

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self::Type>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TelegrandWindow {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }

    impl WidgetImpl for TelegrandWindow {}
    impl WindowImpl for TelegrandWindow {}
    impl ApplicationWindowImpl for TelegrandWindow {}
    impl AdwApplicationWindowImpl for TelegrandWindow {}
}

glib::wrapper! {
    pub struct TelegrandWindow(ObjectSubclass<imp::TelegrandWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow;
}

impl TelegrandWindow {
    pub fn new<P: glib::IsA<gtk::Application>>(app: &P, gtk_receiver: glib::Receiver<telegram::MessageGTK>, tg_sender: mpsc::Sender<telegram::MessageTG>) -> Self {
        let window = glib::Object::new(&[("application", app)])
            .expect("Failed to create TelegrandWindow");

        let self_ = imp::TelegrandWindow::from_instance(&window);
        let add_account_window = &*self_.add_account_window;
        add_account_window.init_signals(&tg_sender);

        let chat_stack = &*self_.chat_stack;
        let dialog_model = self_.dialog_model.clone();
        self_.dialogs_list.connect_row_activated(glib::clone!(@weak chat_stack, @weak dialog_model => move |_, row| {
            let index = row.get_index();
            if let Some(item) = dialog_model.get_object(index as u32) {
                let data = item.downcast_ref::<DialogData>()
                    .expect("Row data is of wrong type");
                let chat_id = data.get_chat_id();
                chat_stack.set_visible_child_name(&chat_id);
            }
        }));

        self_.dialogs_list.bind_model(Some(&self_.dialog_model), move |item| {
            let data = item.downcast_ref::<DialogData>()
                .expect("Row data is of wrong type");
            let row = DialogRow::new(data);
            row.upcast::<gtk::Widget>()
        });

        gtk_receiver.attach(None, glib::clone!(@weak add_account_window, @weak chat_stack, @weak dialog_model => move |msg| {
            match msg {
                telegram::MessageGTK::AccountNotAuthorized =>
                    add_account_window.show(),
                telegram::MessageGTK::NeedConfirmationCode =>
                    add_account_window.navigate_forward(),
                telegram::MessageGTK::SuccessfullySignedIn =>
                    add_account_window.hide(),
                telegram::MessageGTK::LoadDialog(dialog) => {
                    let chat = dialog.chat();
                    let chat_id = chat.id().to_string();
                    let chat_name = chat.name();
                    let last_message = dialog.last_message.as_ref().unwrap().text();
                    dialog_model.append(&DialogData::new(&chat_id, chat_name,
                        last_message));

                    let chat_box = ChatBox::new();
                    chat_stack.add_titled(&chat_box, Some(&chat_id), chat_name);
                }
                telegram::MessageGTK::NewMessage(message) => {
                    let chat = message.chat();
                    let chat_id = chat.id().to_string();
                    let message_text = message.text();
                    let outgoing = message.outgoing();

                    match chat_stack.get_child_by_name(&chat_id) {
                        Some(child) => {
                            let chat_box: ChatBox = child.downcast().unwrap();
                            chat_box.add_message(message_text, outgoing);
                        }
                        None => {
                            let chat_name = chat.name();
                            dialog_model.append(&DialogData::new(&chat_id, chat_name,
                                message_text));

                            let chat_box = ChatBox::new();
                            chat_stack.add_titled(&chat_box, Some(&chat_id), chat_name);
                            chat_box.add_message(message_text, outgoing);
                        }
                    }
                }
            }

            glib::Continue(true)
        }));

        window
    }
}

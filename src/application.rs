use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib};
use gtk_macros::action;

use crate::config;
use crate::preferences_window::PreferencesWindow;
use crate::window::TelegrandWindow;

mod imp {
    use super::*;
    use grammers_client::{Config, InitParams};
    use grammers_session::FileSession;
    use once_cell::sync::OnceCell;
    use tokio::sync::mpsc;

    use crate::telegram;

    #[derive(Debug, Default)]
    pub struct TelegrandApplication {
        pub window: OnceCell<glib::WeakRef<TelegrandWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TelegrandApplication {
        const NAME: &'static str = "TelegrandApplication";
        type Type = super::TelegrandApplication;
        type ParentType = gtk::Application;
    }

    impl ObjectImpl for TelegrandApplication {}

    impl gio::subclass::prelude::ApplicationImpl for TelegrandApplication {
        fn activate(&self, app: &Self::Type) {
            let priv_ = TelegrandApplication::from_instance(app);
            if let Some(window) = priv_.window.get() {
                let window = window.upgrade().unwrap();
                window.show();
                window.present();
                return;
            }

            app.set_resource_base_path(Some("/com/github/melix99/telegrand/"));
            app.setup_css();

            let (gtk_sender, gtk_receiver) = mpsc::channel(20);
            let (tg_sender, tg_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

            let window = TelegrandWindow::new(app, tg_receiver, gtk_sender);
            self.window
                .set(window.downgrade())
                .expect("Window already set");

            let settings = gio::Settings::new(config::APP_ID);
            let server_address = settings.get_string("custom-server-address");
            let params;
            if let Ok(server_address) = server_address.parse() {
                params = InitParams {
                    server_addr: Some(server_address),
                    ..Default::default()
                };
            } else {
                params = Default::default();
            }

            let api_id = config::TG_API_ID.to_owned();
            let api_hash = config::TG_API_HASH.to_owned();
            let config = Config {
                session: FileSession::load_or_create("telegrand.session").unwrap(),
                api_id,
                api_hash: api_hash.clone(),
                params: params,
            };
            telegram::spawn(tg_sender, gtk_receiver, config);

            app.setup_gactions();
            app.get_main_window().present();
        }

        fn startup(&self, app: &Self::Type) {
            self.parent_startup(app);
        }
    }

    impl GtkApplicationImpl for TelegrandApplication {}
}

glib::wrapper! {
    pub struct TelegrandApplication(ObjectSubclass<imp::TelegrandApplication>)
        @extends gio::Application, gtk::Application, @implements gio::ActionMap, gio::ActionGroup;
}

impl TelegrandApplication {
    pub fn new() -> Self {
        glib::Object::new(&[
            ("application-id", &Some(config::APP_ID)),
            ("flags", &gio::ApplicationFlags::empty()),
        ])
        .expect("Failed to create TelegrandApplication")
    }

    fn get_main_window(&self) -> TelegrandWindow {
        let priv_ = imp::TelegrandApplication::from_instance(self);
        priv_.window.get().unwrap().upgrade().unwrap()
    }

    fn setup_gactions(&self) {
        // Preferences
        action!(
            self,
            "preferences",
            glib::clone!(@weak self as app => move |_, _| {
                app.show_preferences_window();
            })
        );

        // About
        action!(
            self,
            "about",
            glib::clone!(@weak self as app => move |_, _| {
                app.show_about_dialog();
            })
        );
    }

    fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/com/github/melix99/telegrand/style.css");
        if let Some(display) = gdk::Display::get_default() {
            gtk::StyleContext::add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn show_about_dialog(&self) {
        let about_dialog = gtk::AboutDialogBuilder::new()
            .program_name("Telegrand")
            .license_type(gtk::License::Gpl30)
            .website("https://github.com/melix99/telegrand/")
            .version(config::VERSION)
            .transient_for(&self.get_main_window())
            .modal(true)
            .authors(vec!["Marco Melorio".into()])
            .artists(vec!["Marco Melorio".into()])
            .build();
        about_dialog.show();
    }

    fn show_preferences_window(&self) {
        let preferences_window = PreferencesWindow::new();
        preferences_window.set_transient_for(Some(&self.get_main_window()));
        preferences_window.show();
    }

    pub fn run(&self) {
        ApplicationExtManual::run(self);
    }
}

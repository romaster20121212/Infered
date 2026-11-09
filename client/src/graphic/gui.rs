 
 
 
 
 
 

use crate::graphic::anim::{self, Easing};
use crate::graphic::input::{close_gui_if_requested, GUI_OPEN};
use crate::graphic::{
    configs, crosshair, edit_ui, esp, friends, hud, menu, notification, pause_menu, player_card,
    settings, theme, title_logo,
};
#[cfg(feature = "devtest")]
use crate::graphic::{dev_popup, dev_tester};
use crate::module::performance::menu_esp_pause;
use egui::{Context, Id};
use std::sync::atomic::Ordering;

 
pub fn render_all(ctx: &Context) {
    close_gui_if_requested();

     
     
     
    edit_ui::sync();
    #[cfg(feature = "devtest")]
    dev_tester::sync_cursor();
     
     
     
    #[cfg(feature = "devtest")]
    dev_tester::pump();

     
     
    if theme::take_style_dirty() {
        theme::apply(ctx);
    }

     
     
    crate::graphic::lyrics_world::prepare_sky(ctx);
    let open = GUI_OPEN.load(Ordering::Relaxed);

     
     
     
    settings::sync_with_menu(open);
    configs::sync_with_menu(open);
    crate::graphic::clarity_menu::sync_with_menu(open);

     
     
     
    if !(open && menu_esp_pause::pauses_esp()) {
        esp::draw(ctx);
    }

     
     
    title_logo::draw(ctx);

    let progress = anim::toggle(ctx, Id::new("clickgui_open"), open, 0.18, Easing::InOut);

     
     
    hud::draw(ctx, progress <= 0.0);

     
     
    crosshair::draw(ctx);

     
     
     
    pause_menu::draw(ctx);

     
     
     
     
    let settings_progress = settings::progress(ctx, progress);
    let settings_visible = settings::is_visible(settings_progress);
    let friends_progress = friends::progress(ctx, progress);
    let friends_visible = friends::is_visible(friends_progress);
    let configs_progress = configs::progress(ctx, progress);
    let configs_visible = configs::is_visible(configs_progress);
    let modal_visible = settings_visible || friends_visible || configs_visible;

    if progress > 0.0 {
         
         
        menu::draw(
            ctx,
            progress,
            settings_progress,
            friends_progress,
            configs_progress,
        );
         
         
         
        if modal_visible {
            player_card::close_menu();
        } else {
            player_card::draw(ctx, progress);
        }
    }

     
    settings::draw(ctx, settings_progress);
    friends::draw(ctx, friends_progress);
    configs::draw(ctx, configs_progress);

     
     
     
    if !modal_visible {
        edit_ui::draw(ctx);
    }

     
     
     
    #[cfg(feature = "devtest")]
    {
        dev_popup::draw(ctx);
        dev_tester::draw(ctx);
    }

    notification::draw(ctx);

     
    anim::gc(ctx);
}

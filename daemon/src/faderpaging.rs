use goxlr_ipc::GoXLRCommand;
use goxlr_types::{ChannelName, FaderName, SampleBank};
use log::debug;
use crate::device::Device;

#[derive(Debug, Clone)]
struct FaderItem {
    id: FaderName,
    channel: ChannelName,
    gradient: bool,
    meter: bool,
}

#[derive(Debug, Clone)]
struct Page {
    id: SampleBank,
    fader_a: FaderItem,
    fader_b: FaderItem,
    fader_c: FaderItem,
    fader_d: FaderItem,
}

#[derive(Debug)]
pub struct FaderPaging<'a> {
    pages: Vec<Page>,
    _bullshit_device_settings_bs_marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> FaderPaging<'a> {
    pub fn new() -> Self {
        let pages = vec![
            Page {
                id: SampleBank::A,
                fader_a: FaderItem {
                    id: FaderName::A,
                    channel: ChannelName::Mic,
                    gradient: true,
                    meter: false,
                },
                fader_b: FaderItem {
                    id: FaderName::B,
                    channel: ChannelName::Chat,
                    gradient: true,
                    meter: false,
                },
                fader_c: FaderItem {
                    id: FaderName::C,
                    channel: ChannelName::Music,
                    gradient: true,
                    meter: false,
                },
                fader_d: FaderItem {
                    id: FaderName::D,
                    channel: ChannelName::Game,
                    gradient: true,
                    meter: false,
                },
            },
            Page {
                id: SampleBank::B,
                fader_a: FaderItem {
                    id: FaderName::A,
                    channel: ChannelName::Mic,
                    gradient: true,
                    meter: false,
                },
                fader_b: FaderItem {
                    id: FaderName::B,
                    channel: ChannelName::Music,
                    gradient: true,
                    meter: false,
                },
                fader_c: FaderItem {
                    id: FaderName::C,
                    channel: ChannelName::LineOut,
                    gradient: true,
                    meter: false,
                },
                fader_d: FaderItem {
                    id: FaderName::D,
                    channel: ChannelName::Headphones,
                    gradient: true,
                    meter: false,
                },
            },
            Page {
                id: SampleBank::C,
                fader_a: FaderItem {
                    id: FaderName::A,
                    channel: ChannelName::Console,
                    gradient: true,
                    meter: false,
                },
                fader_b: FaderItem {
                    id: FaderName::B,
                    channel: ChannelName::Sample,
                    gradient: true,
                    meter: false,
                },
                fader_c: FaderItem {
                    id: FaderName::C,
                    channel: ChannelName::LineIn,
                    gradient: true,
                    meter: false,
                },
                fader_d: FaderItem {
                    id: FaderName::D,
                    channel: ChannelName::System,
                    gradient: true,
                    meter: false,
                },
            },
        ];
        Self { pages, _bullshit_device_settings_bs_marker: std::marker::PhantomData }
    }

    pub async fn change_page(&self, device: &mut Device<'a>, bank: &SampleBank) {
        if let Some(page) = self.pages.iter().find(|p| p.id == *bank) {
            self.apply_page(device, page).await;
        }
        debug!("Fader page changed to {:?}", bank);
    }

    pub async fn setup_pages(&self, device: &mut Device<'a>) {
        for page in self.pages.iter().rev() {
            self.apply_page(device, page).await;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        debug!("Fader paging setup complete.");
    }

    async fn apply_page(&self, device: &mut Device<'a>, page: &Page) {
        self.apply_fader_item(device, &page.fader_a).await;
        self.apply_fader_item(device, &page.fader_b).await;
        self.apply_fader_item(device, &page.fader_c).await;
        self.apply_fader_item(device, &page.fader_d).await;
    }

    async fn apply_fader_item(&self, device: &mut Device<'a>, fader_item: &FaderItem) {
        device.perform_command(GoXLRCommand::SetFader(fader_item.id, fader_item.channel)).await.unwrap();
        //device.perform_command(GoXLRCommand::SetFaderColours(fader_item.id, (), ())).await.unwrap();
        device.perform_command(GoXLRCommand::SetScribbleText(fader_item.id, fader_item.channel.to_string())).await.unwrap();
        device.perform_command(GoXLRCommand::SetScribbleNumber(fader_item.id, String::from("Hello"))).await.unwrap();
        device.perform_command(GoXLRCommand::SetScribbleInvert(fader_item.id, true)).await.unwrap();
    }
}
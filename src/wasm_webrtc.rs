//! WASM用WebRTC P2P通信

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm")]
use web_sys::{
    RtcConfiguration, RtcDataChannel, RtcIceCandidate, RtcIceCandidateInit,
    RtcPeerConnection, RtcSessionDescription, RtcSessionDescriptionInit, RtcSdpType,
};
#[cfg(feature = "wasm")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "wasm")]
use std::collections::HashMap;

/// WebRTC接続状態
#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPeerConnection {
    pub peer_id: String,
    pub connection_state: String,
}

/// WebRTCマネージャー
#[cfg(feature = "wasm")]
pub struct WasmWebRtcManager {
    pub local_id: String,
    pub connections: HashMap<String, RtcPeerConnection>,
    pub data_channels: HashMap<String, RtcDataChannel>,
}

#[cfg(feature = "wasm")]
impl WasmWebRtcManager {
    pub fn new(local_id: String) -> Self {
        Self {
            local_id,
            connections: HashMap::new(),
            data_channels: HashMap::new(),
        }
    }

    /// 新しいピア接続を作成
    pub fn create_peer_connection(&mut self, peer_id: String) -> Result<(), String> {
        let mut config = RtcConfiguration::new();
        // STUNサーバーの設定（無料の公開STUNサーバーを使用）
        let ice_servers = js_sys::Array::new();
        let stun_server = js_sys::Object::new();
        js_sys::Reflect::set(
            &stun_server,
            &JsValue::from_str("urls"),
            &JsValue::from_str("stun:stun.l.google.com:19302"),
        )
        .map_err(|_| "Failed to set STUN server".to_string())?;
        ice_servers.push(&stun_server);
        config.ice_servers(&ice_servers);

        let pc = RtcPeerConnection::new_with_configuration(&config)
            .map_err(|_| "Failed to create peer connection".to_string())?;

        self.connections.insert(peer_id.clone(), pc);

        Ok(())
    }

    /// データチャネルを作成
    pub fn create_data_channel(&mut self, peer_id: String, label: &str) -> Result<(), String> {
        let pc = self
            .connections
            .get(&peer_id)
            .ok_or("Peer connection not found".to_string())?;

        let dc = pc.create_data_channel(label);
        self.data_channels.insert(peer_id, dc);

        Ok(())
    }

    /// オファーを作成
    pub async fn create_offer(&self, peer_id: &str) -> Result<String, String> {
        let pc = self
            .connections
            .get(peer_id)
            .ok_or("Peer connection not found".to_string())?;

        let offer = wasm_bindgen_futures::JsFuture::from(pc.create_offer())
            .await
            .map_err(|_| "Failed to create offer".to_string())?;

        let offer_sdp = js_sys::Reflect::get(&offer, &JsValue::from_str("sdp"))
            .map_err(|_| "Failed to get SDP".to_string())?
            .as_string()
            .ok_or("SDP is not a string".to_string())?;

        let mut desc_init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        desc_init.sdp(&offer_sdp);

        wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&desc_init))
            .await
            .map_err(|_| "Failed to set local description".to_string())?;

        Ok(offer_sdp)
    }

    /// アンサーを作成
    pub async fn create_answer(&self, peer_id: &str, offer_sdp: &str) -> Result<String, String> {
        let pc = self
            .connections
            .get(peer_id)
            .ok_or("Peer connection not found".to_string())?;

        let mut offer_init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_init.sdp(offer_sdp);

        wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&offer_init))
            .await
            .map_err(|_| "Failed to set remote description".to_string())?;

        let answer = wasm_bindgen_futures::JsFuture::from(pc.create_answer())
            .await
            .map_err(|_| "Failed to create answer".to_string())?;

        let answer_sdp = js_sys::Reflect::get(&answer, &JsValue::from_str("sdp"))
            .map_err(|_| "Failed to get SDP".to_string())?
            .as_string()
            .ok_or("SDP is not a string".to_string())?;

        let mut answer_init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_init.sdp(&answer_sdp);

        wasm_bindgen_futures::JsFuture::from(pc.set_local_description(&answer_init))
            .await
            .map_err(|_| "Failed to set local description".to_string())?;

        Ok(answer_sdp)
    }

    /// アンサーを設定
    pub async fn set_answer(&self, peer_id: &str, answer_sdp: &str) -> Result<(), String> {
        let pc = self
            .connections
            .get(peer_id)
            .ok_or("Peer connection not found".to_string())?;

        let mut answer_init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_init.sdp(answer_sdp);

        wasm_bindgen_futures::JsFuture::from(pc.set_remote_description(&answer_init))
            .await
            .map_err(|_| "Failed to set remote description".to_string())?;

        Ok(())
    }

    /// ICE候補を追加
    pub async fn add_ice_candidate(
        &self,
        peer_id: &str,
        candidate: &str,
        sdp_mid: &str,
        sdp_m_line_index: u16,
    ) -> Result<(), String> {
        let pc = self
            .connections
            .get(peer_id)
            .ok_or("Peer connection not found".to_string())?;

        let mut ice_init = RtcIceCandidateInit::new(candidate);
        ice_init.sdp_mid(Some(sdp_mid));
        ice_init.sdp_m_line_index(Some(sdp_m_line_index));

        wasm_bindgen_futures::JsFuture::from(pc.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&ice_init)))
            .await
            .map_err(|_| "Failed to add ICE candidate".to_string())?;

        Ok(())
    }

    /// データを送信
    pub fn send_data(&self, peer_id: &str, data: &str) -> Result<(), String> {
        let dc = self
            .data_channels
            .get(peer_id)
            .ok_or("Data channel not found".to_string())?;

        dc.send_with_str(data)
            .map_err(|_| "Failed to send data".to_string())?;

        Ok(())
    }

    /// 全ピアに送信
    pub fn broadcast(&self, data: &str) -> Result<(), String> {
        for (peer_id, dc) in &self.data_channels {
            if let Err(e) = dc.send_with_str(data) {
                eprintln!("Failed to send to {}: {:?}", peer_id, e);
            }
        }
        Ok(())
    }

    /// 接続を閉じる
    pub fn close_connection(&mut self, peer_id: &str) -> Result<(), String> {
        if let Some(dc) = self.data_channels.remove(peer_id) {
            dc.close();
        }

        if let Some(pc) = self.connections.remove(peer_id) {
            pc.close();
        }

        Ok(())
    }

    /// 全接続を閉じる
    pub fn close_all(&mut self) {
        for (_, dc) in self.data_channels.drain() {
            dc.close();
        }

        for (_, pc) in self.connections.drain() {
            pc.close();
        }
    }
}

/// シグナリングデータ
#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalingData {
    Offer { sdp: String },
    Answer { sdp: String },
    IceCandidate { candidate: String, sdp_mid: String, sdp_m_line_index: u16 },
}

#[cfg(feature = "wasm")]
impl SignalingData {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

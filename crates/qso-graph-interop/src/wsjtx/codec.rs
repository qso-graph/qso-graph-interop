// SPDX-License-Identifier: GPL-3.0-or-later
#![forbid(unsafe_code)]
use super::{
    Error,
    qt::{Reader, Writer},
    types::*,
};
pub(super) fn known(n: u32) -> bool {
    n <= 16
}
pub(super) fn read_body(r: &mut Reader<'_>, n: u32) -> Result<Body, Error> {
    Ok(match n {
        0 => Body::Heartbeat(Heartbeat {
            max_schema: r.optional(|r| r.u32())?,
            version: r.optional(|r| r.utf8())?,
            revision: r.optional(|r| r.utf8())?,
        }),
        1 => Body::Status(Status {
            dial_frequency_hz: r.u64()?,
            mode: r.utf8()?,
            dx_call: r.utf8()?,
            report: r.utf8()?,
            tx_mode: r.utf8()?,
            tx_enabled: r.boolean()?,
            transmitting: r.boolean()?,
            decoding: r.boolean()?,
            rx_df_hz: r.u32()?,
            tx_df_hz: r.u32()?,
            de_call: r.optional(|r| r.utf8())?,
            de_grid: r.optional(|r| r.utf8())?,
            dx_grid: r.optional(|r| r.utf8())?,
            tx_watchdog: r.optional(|r| r.boolean())?,
            sub_mode: r.optional(|r| r.utf8())?,
            fast_mode: r.optional(|r| r.boolean())?,
            special_operation_mode: r.optional(|r| r.u8())?,
            frequency_tolerance_hz: r.optional(|r| r.u32())?,
            tr_period_seconds: r.optional(|r| r.u32())?,
            configuration_name: r.optional(|r| r.utf8())?,
            tx_message: r.optional(|r| r.utf8())?,
        }),
        2 => Body::Decode(Decode {
            new: r.boolean()?,
            time: r.qtime()?,
            snr_db: r.i32()?,
            delta_time_seconds: r.f64()?,
            delta_frequency_hz: r.u32()?,
            mode: r.utf8()?,
            message: r.utf8()?,
            low_confidence: r.optional(|r| r.boolean())?,
            off_air: r.optional(|r| r.boolean())?,
        }),
        3 => Body::Clear(Clear {
            window: r.optional(|r| r.u8())?,
        }),
        4 => Body::Reply(Reply {
            time: r.qtime()?,
            snr_db: r.i32()?,
            delta_time_seconds: r.f64()?,
            delta_frequency_hz: r.u32()?,
            mode: r.utf8()?,
            message: r.utf8()?,
            low_confidence: r.optional(|r| r.boolean())?,
            modifiers: r.optional(|r| r.u8())?,
        }),
        5 => Body::QsoLogged(Box::new(QsoLogged {
            time_off: r.qdatetime()?,
            dx_call: r.utf8()?,
            dx_grid: r.utf8()?,
            frequency_hz: r.u64()?,
            mode: r.utf8()?,
            report_sent: r.utf8()?,
            report_received: r.utf8()?,
            tx_power: r.utf8()?,
            comments: r.utf8()?,
            name: r.utf8()?,
            time_on: r.optional(|r| r.qdatetime())?,
            operator_call: r.optional(|r| r.utf8())?,
            my_call: r.optional(|r| r.utf8())?,
            my_grid: r.optional(|r| r.utf8())?,
            exchange_sent: r.optional(|r| r.utf8())?,
            exchange_received: r.optional(|r| r.utf8())?,
            propagation_mode: r.optional(|r| r.utf8())?,
        })),
        8 => Body::HaltTx(HaltTx {
            auto_tx_only: r.boolean()?,
        }),
        9 => Body::FreeText(FreeText {
            text: r.utf8()?,
            send: r.boolean()?,
        }),
        10 => Body::WsprDecode(WsprDecode {
            new: r.boolean()?,
            time: r.qtime()?,
            snr_db: r.i32()?,
            delta_time_seconds: r.f64()?,
            frequency_hz: r.u64()?,
            drift_hz: r.i32()?,
            callsign: r.utf8()?,
            grid: r.utf8()?,
            power_dbm: r.i32()?,
            off_air: r.optional(|r| r.boolean())?,
        }),
        11 => Body::Location(Location {
            location: r.utf8()?,
        }),
        12 => Body::LoggedAdif(LoggedAdif { adif: r.utf8()? }),
        13 => Body::HighlightCallsign(HighlightCallsign {
            callsign: r.utf8()?,
            background: r.qcolor()?,
            foreground: r.qcolor()?,
            highlight_last: r.optional(|r| r.boolean())?,
        }),
        14 => Body::SwitchConfiguration(SwitchConfiguration {
            configuration_name: r.utf8()?,
        }),
        15 => Body::Configure(Configure {
            mode: r.utf8()?,
            frequency_tolerance_hz: r.u32()?,
            sub_mode: r.utf8()?,
            fast_mode: r.boolean()?,
            tr_period_seconds: r.u32()?,
            rx_df_hz: r.u32()?,
            dx_call: r.utf8()?,
            dx_grid: r.utf8()?,
            generate_messages: r.optional(|r| r.boolean())?,
        }),
        16 => Body::AnnotationInfo(AnnotationInfo {
            dx_call: r.utf8()?,
            sort_order_provided: r.boolean()?,
            sort_order: r.u32()?,
        }),
        6 => Body::Close,
        7 => Body::Replay,
        _ => unreachable!("known message type"),
    })
}
pub(super) fn write_body(w: &mut Writer, body: &Body) -> Result<(), Error> {
    match body {
        Body::Heartbeat(v) => {
            w.optional(&v.max_schema, |w, v| w.u32(*v))?;
            w.optional(&v.version, |w, v| w.utf8(v))?;
            w.optional(&v.revision, |w, v| w.utf8(v))?;
        }
        Body::Status(v) => {
            w.u64(v.dial_frequency_hz)?;
            w.utf8(&v.mode)?;
            w.utf8(&v.dx_call)?;
            w.utf8(&v.report)?;
            w.utf8(&v.tx_mode)?;
            w.boolean(v.tx_enabled)?;
            w.boolean(v.transmitting)?;
            w.boolean(v.decoding)?;
            w.u32(v.rx_df_hz)?;
            w.u32(v.tx_df_hz)?;
            w.optional(&v.de_call, |w, v| w.utf8(v))?;
            w.optional(&v.de_grid, |w, v| w.utf8(v))?;
            w.optional(&v.dx_grid, |w, v| w.utf8(v))?;
            w.optional(&v.tx_watchdog, |w, v| w.boolean(*v))?;
            w.optional(&v.sub_mode, |w, v| w.utf8(v))?;
            w.optional(&v.fast_mode, |w, v| w.boolean(*v))?;
            w.optional(&v.special_operation_mode, |w, v| w.u8(*v))?;
            w.optional(&v.frequency_tolerance_hz, |w, v| w.u32(*v))?;
            w.optional(&v.tr_period_seconds, |w, v| w.u32(*v))?;
            w.optional(&v.configuration_name, |w, v| w.utf8(v))?;
            w.optional(&v.tx_message, |w, v| w.utf8(v))?;
        }
        Body::Decode(v) => {
            w.boolean(v.new)?;
            w.qtime(&v.time)?;
            w.i32(v.snr_db)?;
            w.f64(v.delta_time_seconds)?;
            w.u32(v.delta_frequency_hz)?;
            w.utf8(&v.mode)?;
            w.utf8(&v.message)?;
            w.optional(&v.low_confidence, |w, v| w.boolean(*v))?;
            w.optional(&v.off_air, |w, v| w.boolean(*v))?;
        }
        Body::Clear(v) => {
            w.optional(&v.window, |w, v| w.u8(*v))?;
        }
        Body::Reply(v) => {
            w.qtime(&v.time)?;
            w.i32(v.snr_db)?;
            w.f64(v.delta_time_seconds)?;
            w.u32(v.delta_frequency_hz)?;
            w.utf8(&v.mode)?;
            w.utf8(&v.message)?;
            w.optional(&v.low_confidence, |w, v| w.boolean(*v))?;
            w.optional(&v.modifiers, |w, v| w.u8(*v))?;
        }
        Body::QsoLogged(v) => {
            w.qdatetime(&v.time_off)?;
            w.utf8(&v.dx_call)?;
            w.utf8(&v.dx_grid)?;
            w.u64(v.frequency_hz)?;
            w.utf8(&v.mode)?;
            w.utf8(&v.report_sent)?;
            w.utf8(&v.report_received)?;
            w.utf8(&v.tx_power)?;
            w.utf8(&v.comments)?;
            w.utf8(&v.name)?;
            w.optional(&v.time_on, |w, v| w.qdatetime(v))?;
            w.optional(&v.operator_call, |w, v| w.utf8(v))?;
            w.optional(&v.my_call, |w, v| w.utf8(v))?;
            w.optional(&v.my_grid, |w, v| w.utf8(v))?;
            w.optional(&v.exchange_sent, |w, v| w.utf8(v))?;
            w.optional(&v.exchange_received, |w, v| w.utf8(v))?;
            w.optional(&v.propagation_mode, |w, v| w.utf8(v))?;
        }
        Body::HaltTx(v) => {
            w.boolean(v.auto_tx_only)?;
        }
        Body::FreeText(v) => {
            w.utf8(&v.text)?;
            w.boolean(v.send)?;
        }
        Body::WsprDecode(v) => {
            w.boolean(v.new)?;
            w.qtime(&v.time)?;
            w.i32(v.snr_db)?;
            w.f64(v.delta_time_seconds)?;
            w.u64(v.frequency_hz)?;
            w.i32(v.drift_hz)?;
            w.utf8(&v.callsign)?;
            w.utf8(&v.grid)?;
            w.i32(v.power_dbm)?;
            w.optional(&v.off_air, |w, v| w.boolean(*v))?;
        }
        Body::Location(v) => {
            w.utf8(&v.location)?;
        }
        Body::LoggedAdif(v) => {
            w.utf8(&v.adif)?;
        }
        Body::HighlightCallsign(v) => {
            w.utf8(&v.callsign)?;
            w.qcolor(&v.background)?;
            w.qcolor(&v.foreground)?;
            w.optional(&v.highlight_last, |w, v| w.boolean(*v))?;
        }
        Body::SwitchConfiguration(v) => {
            w.utf8(&v.configuration_name)?;
        }
        Body::Configure(v) => {
            w.utf8(&v.mode)?;
            w.u32(v.frequency_tolerance_hz)?;
            w.utf8(&v.sub_mode)?;
            w.boolean(v.fast_mode)?;
            w.u32(v.tr_period_seconds)?;
            w.u32(v.rx_df_hz)?;
            w.utf8(&v.dx_call)?;
            w.utf8(&v.dx_grid)?;
            w.optional(&v.generate_messages, |w, v| w.boolean(*v))?;
        }
        Body::AnnotationInfo(v) => {
            w.utf8(&v.dx_call)?;
            w.boolean(v.sort_order_provided)?;
            w.u32(v.sort_order)?;
        }
        Body::Close | Body::Replay => {}
    }
    Ok(())
}

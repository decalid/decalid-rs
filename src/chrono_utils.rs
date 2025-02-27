pub trait TimeDeltaToString {
    fn to_iso8601_string(&self) -> String;
}

impl TimeDeltaToString for chrono::Duration {
    fn to_iso8601_string(&self) -> String {
        let total_seconds = self.num_seconds();
        let seconds = total_seconds % 60;
        let total_minutes = total_seconds / 60;
        let minutes = total_minutes % 60;
        let total_hours = total_minutes / 60;
        let hours = total_hours % 24;
        let days = total_hours / 24;

        let mut result = String::from("P");

        if days > 0 {
            result.push_str(&format!("{}D", days));
        }

        if hours > 0 || minutes > 0 || seconds > 0 {
            result.push('T');
            if hours > 0 {
                result.push_str(&format!("{}H", hours));
            }
            if minutes > 0 {
                result.push_str(&format!("{}M", minutes));
            }
            if seconds > 0 {
                result.push_str(&format!("{}S", seconds));
            }
        }

        result
    }
}

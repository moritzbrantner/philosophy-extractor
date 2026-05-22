use philosophy_extractor_schema::UnifiedWorldviewV10;

pub fn export_worldview_json(worldview: &UnifiedWorldviewV10) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(worldview)
}

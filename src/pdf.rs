/// Merge one or more single-page PDFs into one multi-page PDF.
/// Each element of `pages` must be a valid PDF byte slice.
pub fn merge_pdf_pages(pages: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    if pages.is_empty() {
        return Err("no pages to merge".into());
    }

    if pages.len() == 1 {
        return Ok(pages[0].clone());
    }

    let mut base = lopdf::Document::load_mem(&pages[0])
        .map_err(|e| format!("load first PDF: {}", e))?;

    for (i, page_bytes) in pages.iter().enumerate().skip(1) {
        let doc = lopdf::Document::load_mem(page_bytes)
            .map_err(|e| format!("load PDF page {}: {}", i, e))?;

        let src_page_ids: Vec<lopdf::ObjectId> = doc.get_pages().values().copied().collect();

        let mut object_map: std::collections::HashMap<lopdf::ObjectId, lopdf::ObjectId> =
            std::collections::HashMap::new();

        for (obj_id, obj) in doc.objects.iter() {
            let new_id = base.add_object(obj.clone());
            object_map.insert(*obj_id, new_id);
        }

        for page_id in src_page_ids {
            if let Some(&new_page_id) = object_map.get(&page_id) {
                // Get Pages object ID from catalog
                let catalog_dict = base.catalog();
                if let Ok(dict) = catalog_dict {
                    let pages_ref = dict.get(b"Pages").ok()
                        .and_then(|o| {
                            if let lopdf::Object::Reference(id) = o {
                                Some(*id)
                            } else {
                                None
                            }
                        });

                    if let Some(pages_id) = pages_ref {
                        if let Ok(obj) = base.get_object_mut(pages_id) {
                            if let lopdf::Object::Dictionary(ref mut dict) = obj {
                                if let Ok(kids_val) = dict.get_mut(b"Kids") {
                                    if let lopdf::Object::Array(ref mut kids) = kids_val {
                                        kids.push(lopdf::Object::Reference(new_page_id));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    base.compress();

    let mut output = Vec::new();
    base.save_to(&mut output)
        .map_err(|e| format!("save merged PDF: {}", e))?;

    Ok(output)
}

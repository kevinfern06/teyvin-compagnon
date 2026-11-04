//! Teyvin Compagnon : reconnaître l'inventaire et les personnages sans connaître les numéros de la version.
//!
//! À chaque version du jeu, les deux paquets changent de numéro de commande, et leur liste de numéro de champ. Le
//! moteur les a en dur : le compagnon ne lit donc plus le compte tant qu'il n'a pas été mis à jour. La forme des
//! éléments, elle, ne bouge pas. On cherche donc la liste à sa forme, et on retient le numéro du premier paquet
//! trouvé : le jeu envoie le compte entier à la connexion, avant toute liste partielle (objets reçus, etc.).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};

use protobuf::Message;
use protobuf::UnknownValueRef::LengthDelimited;
use tracing::info;

use crate::r#gen::protos::{AvatarInfo, Item, Unk};

/// En dessous, ce n'est pas l'inventaire entier d'un compte (les listes partielles ont la même forme).
const MIN_ITEMS: usize = 50;
/// Une équipe complète : écarte les paquets qui ne décrivent qu'un perso.
const MIN_AVATARS: usize = 4;
/// Évite de décoder les innombrables petits paquets du jeu.
const MIN_BYTES: usize = 500;

static ITEMS_ID: AtomicU16 = AtomicU16::new(0);
static AVATARS_ID: AtomicU16 = AtomicU16::new(0);

/// La plus longue liste d'éléments valides d'un message dont on ignore les numéros de champ.
/// Une liste dont plus d'un élément sur dix n'est pas valide est un autre type de données.
fn longest_list<T: Message>(data: &[u8], valid: impl Fn(&T) -> bool) -> Vec<T> {
    let Ok(message) = Unk::parse_from_bytes(data) else {
        return Vec::new();
    };
    let mut fields: HashMap<u32, (usize, Vec<T>)> = HashMap::new();
    for (number, value) in message.unknown_fields().iter() {
        let LengthDelimited(bytes) = value else {
            continue;
        };
        let (total, found) = fields.entry(number).or_default();
        *total += 1;
        if let Ok(element) = T::parse_from_bytes(bytes) {
            if valid(&element) {
                found.push(element);
            }
        }
    }
    fields
        .into_values()
        .filter(|(total, found)| found.len() * 10 >= total * 9)
        .map(|(_, found)| found)
        .max_by_key(Vec::len)
        .unwrap_or_default()
}

fn recognize<T: Message>(
    id: u16,
    data: &[u8],
    learned: &AtomicU16,
    min: usize,
    label: &str,
    valid: impl Fn(&T) -> bool,
) -> Option<Vec<T>> {
    let known = learned.load(Ordering::Relaxed);
    if (known != 0 && known != id) || data.len() < MIN_BYTES {
        return None;
    }
    let list = longest_list(data, valid);
    if list.len() < min {
        return None;
    }
    if known == 0 {
        learned.store(id, Ordering::Relaxed);
        info!("Teyvin : paquet {label} reconnu, commande {id}");
    }
    Some(list)
}

fn valid_item(item: &Item) -> bool {
    item.item_id != 0 && item.guid != 0 && item.detail.is_some()
}

/// Niveau du perso (4001) : présent sur tous, absent des paquets qui ressemblent à celui-ci.
fn valid_avatar(avatar: &AvatarInfo) -> bool {
    (10_000_000..11_000_000).contains(&avatar.avatar_id) && avatar.guid != 0 && avatar.prop_map.contains_key(&4001)
}

pub fn items(id: u16, data: &[u8]) -> Option<Vec<Item>> {
    recognize(id, data, &ITEMS_ID, MIN_ITEMS, "inventaire", valid_item)
}

pub fn avatars(id: u16, data: &[u8]) -> Option<Vec<AvatarInfo>> {
    recognize(id, data, &AVATARS_ID, MIN_AVATARS, "personnages", valid_avatar)
}

#[cfg(test)]
mod teyvin_tests {
    use super::*;
    use crate::r#gen::protos::{Material, PropValue};

    fn varint(mut value: u64, out: &mut Vec<u8>) {
        while value >= 0x80 {
            out.push(value as u8 | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }

    /// Un message dont le champ `field` répète ces éléments, précédé d'un champ numérique comme l'en-tête du jeu.
    fn packet<T: Message>(field: u64, list: &[T]) -> Vec<u8> {
        let mut out = vec![0x08, 0x01];
        for element in list {
            let bytes = element.write_to_bytes().unwrap();
            varint(field << 3 | 2, &mut out);
            varint(bytes.len() as u64, &mut out);
            out.extend(bytes);
        }
        out
    }

    fn item(id: u32) -> Item {
        let mut item = Item::new();
        item.item_id = id;
        item.guid = 1 << 40 | id as u64;
        let mut material = Material::new();
        material.count = 3;
        item.set_material(material);
        item
    }

    fn avatar(id: u32) -> AvatarInfo {
        let mut avatar = AvatarInfo::new();
        avatar.avatar_id = id;
        avatar.guid = 1 << 40 | id as u64;
        let mut level = PropValue::new();
        level.type_ = 4001;
        level.val = 90;
        avatar.prop_map.insert(4001, level);
        avatar
    }

    fn inventory(count: u32) -> Vec<Item> {
        (0..count).map(|i| item(100_000 + i)).collect()
    }

    fn items_of(id: u16, data: &[u8], learned: &AtomicU16) -> Option<Vec<Item>> {
        recognize(id, data, learned, MIN_ITEMS, "test", valid_item)
    }

    fn avatars_of(id: u16, data: &[u8], learned: &AtomicU16) -> Option<Vec<AvatarInfo>> {
        recognize(id, data, learned, MIN_AVATARS, "test", valid_avatar)
    }

    #[test]
    fn finds_the_inventory_under_any_field_number() {
        for field in [1, 5, 6, 9, 15, 200] {
            let learned = AtomicU16::new(0);
            assert_eq!(items_of(22160, &packet(field, &inventory(60)), &learned).unwrap().len(), 60);
            assert_eq!(learned.load(Ordering::Relaxed), 22160);
        }
    }

    #[test]
    fn finds_the_characters_under_any_field_number() {
        let list: Vec<_> = (0..40).map(|i| avatar(10_000_002 + i)).collect();
        for field in [1, 7, 14] {
            let learned = AtomicU16::new(0);
            assert_eq!(avatars_of(27799, &packet(field, &list), &learned).unwrap().len(), 40);
        }
    }

    #[test]
    fn ignores_partial_lists() {
        let learned = AtomicU16::new(0);
        assert!(items_of(1, &packet(6, &inventory(49)), &learned).is_none());
        assert_eq!(learned.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn ignores_other_data() {
        let learned = AtomicU16::new(0);
        // Forme des hauts faits : des nombres, sans détail d'objet
        let mut achievement = Item::new();
        achievement.item_id = 80014;
        achievement.guid = 3;
        assert!(items_of(1, &packet(6, &vec![achievement; 100]), &learned).is_none());

        assert!(avatars_of(1, &packet(6, &inventory(60)), &learned).is_none());
        let mut no_level = avatar(10_000_002);
        no_level.prop_map.clear();
        assert!(avatars_of(1, &packet(6, &vec![no_level; 60]), &learned).is_none());
        assert_eq!(learned.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn keeps_the_first_command_found() {
        let learned = AtomicU16::new(0);
        let data = packet(6, &inventory(60));
        assert!(items_of(22160, &data, &learned).is_some());
        assert!(items_of(1234, &data, &learned).is_none(), "autre commande, même forme : liste partielle");
        assert!(items_of(22160, &data, &learned).is_some(), "reconnexion : même commande");
    }
}

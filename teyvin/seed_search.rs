//! Teyvin Compagnon : retrouver la clé de session quand le compagnon a démarré après le jeu.
//!
//! La clé dépend d'une graine tirée d'un générateur que le jeu initialise avec l'heure, une seule fois, à sa première
//! connexion. Le moteur retient cette heure pour les reconnexions suivantes : s'il n'a pas vu la première connexion,
//! il cherche autour de l'heure de la reconnexion et ne trouve rien. On cherche donc depuis le lancement du jeu.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use rand_mt::Mt64;

use crate::crypto::new_key_from_seed;
use crate::cs_rand::Random;

/// Connexions déjà faites par le jeu depuis son lancement, au plus.
const DEPTH: usize = 8;
/// Taille des lots de millisecondes distribués aux fils.
const CHUNK: i64 = 16_384;

/// Heure d'initialisation (en ms, de `from_ms` à `to_ms`) et clé qui déchiffre `data`, s'il y en a une.
pub fn search(from_ms: i64, to_ms: i64, server_seeds: &[u64], data: &[u8]) -> Option<(u64, Vec<u8>)> {
    if data.len() < 4 {
        return None;
    }
    let next = AtomicI64::new(from_ms);
    let found = AtomicBool::new(false);
    let result = Mutex::new(None);
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while !found.load(Ordering::Relaxed) {
                    let start = next.fetch_add(CHUNK, Ordering::Relaxed);
                    if start > to_ms {
                        return;
                    }
                    for time in start..(start + CHUNK).min(to_ms + 1) {
                        if let Some(key) = try_time(time, server_seeds, data) {
                            *result.lock().unwrap() = Some((time as u64, key));
                            found.store(true, Ordering::Relaxed);
                            return;
                        }
                    }
                }
            });
        }
    });

    result.into_inner().unwrap()
}

fn try_time(time: i64, server_seeds: &[u64], data: &[u8]) -> Option<Vec<u8>> {
    let mut generator = Random::seeded(time as i32);
    for _ in 0..DEPTH {
        let client_seed = generator.next_safe_uint64();
        for server_seed in server_seeds {
            let seed = client_seed ^ server_seed;
            // Les deux premiers octets suffisent à écarter presque toutes les graines, sans calculer toute la clé
            if starts_with_magic(seed, data) {
                let key = new_key_from_seed(seed);
                let at = |i: usize| data[i] ^ key[i % key.len()];
                if at(data.len() - 2) == 0x89 && at(data.len() - 1) == 0xAB {
                    return Some(key);
                }
            }
        }
    }
    None
}

fn starts_with_magic(seed: u64, data: &[u8]) -> bool {
    let mut first = Mt64::new(seed);
    let mut generator = Mt64::new(first.next_u64());
    let _ = generator.next_u64();
    let word = generator.next_u64().to_be_bytes();
    data[0] ^ word[0] == 0x45 && data[1] ^ word[1] == 0x67
}

#[cfg(test)]
mod teyvin_tests {
    use super::*;

    /// Un paquet tel que le jeu l'enverrait à sa `login`-ième connexion, avec un générateur lancé à `time`.
    fn packet(time: i64, server_seed: u64, login: usize) -> Vec<u8> {
        let mut generator = Random::seeded(time as i32);
        let client_seed = (0..=login).map(|_| generator.next_safe_uint64()).last().unwrap();
        let key = new_key_from_seed(client_seed ^ server_seed);
        let mut data = vec![0x45, 0x67, 0, 1, 0, 0, 0, 0, 0, 2, 7, 7, 0x89, 0xAB];
        for (i, b) in data.iter_mut().enumerate() {
            *b ^= key[i % key.len()];
        }
        data
    }

    #[test]
    fn finds_the_key_of_a_later_login() {
        let started = 1_793_000_000_000i64;
        let time = started + 95_123;
        let data = packet(time, 42, 2);
        let (found, key) = search(started, started + 3_600_000, &[7, 42], &data).unwrap();
        assert_eq!(found as i64, time);
        assert_eq!(data[0] ^ key[0], 0x45);
    }

    #[test]
    fn finds_nothing_outside_the_window() {
        let started = 1_793_000_000_000i64;
        let data = packet(started - 500_000, 42, 0);
        assert!(search(started, started + 20_000, &[42], &data).is_none());
    }
}

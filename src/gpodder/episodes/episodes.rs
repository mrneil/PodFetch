use actix_web::{HttpResponse, Responder, web};

use actix_web::{get,post};
use actix_web::web::Data;
use crate::db::DB;
use crate::DbPool;
use crate::models::episode::{Episode, EpisodeAction, EpisodeDto};
use crate::models::models::PodcastWatchedPostModel;
use std::borrow::Borrow;
use chrono::NaiveDateTime;
use crate::models::session::Session;
use crate::utils::time::{get_current_timestamp};

#[derive(Serialize, Deserialize)]
pub struct EpisodeActionResponse{
    actions: Vec<Episode>,
    timestamp: i64
}


#[derive(Serialize, Deserialize)]
pub struct EpisodeActionPostResponse{
    update_urls: Vec<String>,
    timestamp: i64
}

#[derive(Serialize, Deserialize)]
pub struct EpisodeSinceRequest{
    since: i64
}

#[get("/episodes/{username}.json")]
pub async fn get_episode_actions(username: web::Path<String>, pool: Data<DbPool>,
                                 opt_flag: Option<web::ReqData<Session>>,
                                 since: web::Query<EpisodeSinceRequest>) -> impl Responder {
    match opt_flag {
        Some(flag) => {
            let username = username.clone();
            if flag.username != username.clone() {
                return HttpResponse::Unauthorized().finish();
            }

            let since_date = NaiveDateTime::from_timestamp_opt(since.since as i64, 0);
            let actions = Episode::get_actions_by_username(username.clone(), &mut *pool.get().unwrap(), since_date)
                .await;
            HttpResponse::Ok().json(EpisodeActionResponse {
                actions,
                timestamp: get_current_timestamp()
            })
        }
        None => {
            HttpResponse::Unauthorized().finish()
        }
    }
}


#[post("/episodes/{username}.json")]
pub async fn upload_episode_actions(username: web::Path<String>, podcast_episode: web::Json<Vec<EpisodeDto>>,opt_flag: Option<web::ReqData<Session>>, conn: Data<DbPool>) -> impl
Responder {
    match opt_flag {
        Some(flag) => {
            if flag.username != username.clone() {
                return HttpResponse::Unauthorized().finish();
            }
            let mut inserted_episodes: Vec<Episode> = vec![];
            podcast_episode.iter().for_each(|episode| {
                let episode = Episode::convert_to_episode(episode, username.clone());
                inserted_episodes.push(Episode::insert_episode(episode.borrow(), &mut *conn.get().unwrap())
                    .expect("Unable to insert episode"));

                if EpisodeAction::from_string(&episode.clone().action) == EpisodeAction::Play {
                    let mut episode_url = episode.clone().episode;
                    // Sometimes podcast provider like to check which browser access their podcast
                    let mut first_split = episode.episode.split("?");
                    let res = first_split.next();

                    if res.is_some() {
                        episode_url = res.unwrap().parse().unwrap()
                    }

                    let podcast_episode = DB::query_podcast_episode_by_url(&mut *conn.get().unwrap(),
                                                                           &*episode_url);
                    if podcast_episode.clone().unwrap().is_none() {
                        return;
                    }

                    let model = PodcastWatchedPostModel {
                        podcast_episode_id: podcast_episode.clone().unwrap().unwrap().episode_id,
                        time: episode.position.unwrap() as i32,
                    };
                    DB::log_watchtime(&mut *conn.get().unwrap(), model, "admin".to_string())
                        .expect("TODO: panic message");
                    println!("episode: {:?}", episode);
                }
            });
            HttpResponse::Ok().json(EpisodeActionPostResponse {
                update_urls: vec![],
                timestamp: get_current_timestamp()
            })
        }
        None => {
            HttpResponse::Unauthorized().finish()
        }
    }
}
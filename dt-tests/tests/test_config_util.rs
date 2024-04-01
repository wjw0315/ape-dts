use std::{
    env,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use configparser::ini::Ini;
use dt_common::config::{
    extractor_config::ExtractorConfig, sinker_config::SinkerConfig, task_config::TaskConfig,
};

pub struct TestConfigUtil {}

const EXTRACTOR: &str = "extractor";
const SINKER: &str = "sinker";
const RUNTIME: &str = "runtime";
const RESUMER: &str = "resumer";
const TEST_PROJECT: &str = "dt-tests";

#[allow(dead_code)]
impl TestConfigUtil {
    pub const LOG_DIR: &str = "log_dir";
    pub const EXTRACTOR_CHECK_LOG_DIR: &str = "extractor_check_log_dir";
    pub const SINKER_CHECK_LOG_DIR: &str = "sinker_check_log_dir";

    pub fn get_project_root() -> String {
        project_root::get_project_root()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    pub fn get_absolute_path(relative_path: &str) -> String {
        format!(
            "{}/{}/tests/{}",
            project_root::get_project_root().unwrap().to_str().unwrap(),
            TEST_PROJECT,
            relative_path
        )
    }

    // result: (absolute_sub_path, sub_path_dir_name)
    pub fn get_absolute_sub_dir(relative_dir: &str) -> Vec<(String, String)> {
        let mut result_dir: Vec<(String, String)> = vec![];

        let absolute_dir = TestConfigUtil::get_absolute_path(relative_dir);
        let path = PathBuf::from(absolute_dir.as_str());

        let entries = fs::read_dir(path).unwrap();
        for entry in entries {
            if let Ok(sub_path) = entry {
                if sub_path.path().is_dir() {
                    let sub_path_dir = sub_path.file_name().to_string_lossy().to_string();
                    result_dir.push((format!("{}/{}", absolute_dir, sub_path_dir), sub_path_dir));
                }
            }
        }

        result_dir
    }

    pub fn update_task_config_from_env(src_task_config_file: &str, dst_task_config_file: &str) {
        let env_local_file = TestConfigUtil::get_absolute_path(".env.local");
        let env_file = TestConfigUtil::get_absolute_path(".env");

        // environment variable settings in .env.local have higher priority
        if fs::metadata(&env_local_file).is_ok() {
            dotenv::from_path(&env_local_file).unwrap();
        }
        dotenv::from_path(&env_file).unwrap();

        let mut update_configs = Vec::new();
        let ini = Self::load_ini(src_task_config_file);
        for (section, kvs) in ini.get_map().unwrap() {
            for (k, v) in kvs.iter() {
                if v.is_none() {
                    continue;
                }
                for (env_k, env_v) in env::vars() {
                    if *v.as_ref().unwrap() == format!("{{{}}}", env_k) {
                        update_configs.push((section.clone(), k.clone(), env_v.clone()));
                        break;
                    }
                }
            }
        }

        TestConfigUtil::update_task_config(
            &src_task_config_file,
            &dst_task_config_file,
            &update_configs,
        );
    }

    pub fn update_task_config_log_dir(
        src_task_config_file: &str,
        dst_task_config_file: &str,
        project_root: &str,
    ) {
        let config = TaskConfig::new(&src_task_config_file);
        let mut update_configs = Vec::new();

        // runtime/log4rs_file
        let log4rs_file = format!("{}/{}", project_root, config.runtime.log4rs_file);
        update_configs.push((RUNTIME.to_string(), "log4rs_file".to_string(), log4rs_file));

        // runtime/log_dir
        let log_dir = format!("{}/{}", project_root, config.runtime.log_dir);
        update_configs.push((RUNTIME.to_string(), "log_dir".to_string(), log_dir.clone()));

        // resumer/resume_log_dir
        let resume_log_dir = format!("{}/{}", project_root, config.resumer.resume_log_dir);
        update_configs.push((
            RESUMER.to_string(),
            "resume_log_dir".to_string(),
            resume_log_dir,
        ));

        // extractor/check_log_dir
        match config.extractor {
            ExtractorConfig::MysqlCheck { check_log_dir, .. }
            | ExtractorConfig::PgCheck { check_log_dir, .. }
            | ExtractorConfig::MongoCheck { check_log_dir, .. } => {
                let extractor_check_log_dir = format!("{}/{}", project_root, check_log_dir);
                update_configs.push((
                    EXTRACTOR.to_string(),
                    "check_log_dir".to_string(),
                    extractor_check_log_dir,
                ));
            }

            ExtractorConfig::RedisSnapshotFile { file_path } => {
                let file_path = format!("{}/{}", project_root, file_path);
                update_configs.push((EXTRACTOR.to_string(), "file_path".to_string(), file_path));
            }

            _ => {}
        }

        match config.sinker {
            // sinker/check_log_dir
            SinkerConfig::MysqlCheck { check_log_dir, .. }
            | SinkerConfig::PgCheck { check_log_dir, .. }
            | SinkerConfig::MongoCheck { check_log_dir, .. } => {
                let sinker_check_log_dir = if !check_log_dir.is_empty() {
                    format!("{}/{}", project_root, check_log_dir)
                } else {
                    format!("{}/check", log_dir)
                };
                update_configs.push((
                    SINKER.to_string(),
                    "check_log_dir".to_string(),
                    sinker_check_log_dir,
                ));
            }

            // sinker/statistic_log_dir
            SinkerConfig::RedisStatistic {
                statistic_log_dir, ..
            } => {
                let sinker_statistic_log_dir = if !statistic_log_dir.is_empty() {
                    format!("{}/{}", project_root, statistic_log_dir)
                } else {
                    format!("{}/statistic", log_dir)
                };
                update_configs.push((
                    SINKER.to_string(),
                    "statistic_log_dir".to_string(),
                    sinker_statistic_log_dir,
                ));
            }

            _ => {}
        }

        TestConfigUtil::update_task_config(
            &src_task_config_file,
            &dst_task_config_file,
            &update_configs,
        );
    }

    pub fn update_task_config(
        src_task_config_file: &str,
        dst_task_config_file: &str,
        config: &Vec<(String, String, String)>,
    ) {
        let mut ini = Self::load_ini(src_task_config_file);
        for (section, key, value) in config.iter() {
            ini.set(section, key, Some(value.to_string()));
        }

        let path = Path::new(&dst_task_config_file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        File::create(&dst_task_config_file)
            .unwrap()
            .set_len(0)
            .unwrap();
        ini.write(dst_task_config_file).unwrap();
    }

    fn load_ini(task_config_file: &str) -> Ini {
        let mut config_str = String::new();
        File::open(task_config_file)
            .unwrap()
            .read_to_string(&mut config_str)
            .unwrap();
        let mut ini = Ini::new();
        ini.read(config_str).unwrap();
        ini
    }
}

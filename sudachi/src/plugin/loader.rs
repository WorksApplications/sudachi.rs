/*
 *  Copyright (c) 2021 Works Applications Co., Ltd.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

/*
 *  Copyright (c) 2021 Works Applications Co., Ltd.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use crate::config::{Config, ConfigError};
use crate::dic::grammar::Grammar;
use crate::error::{SudachiError, SudachiResult};
use libloading::{Library, Symbol};
use serde_json::Value;
use std::path::Path;

pub struct PluginContainer<T: PluginCategory + ?Sized> {
    libraries: Vec<Library>,
    plugins: Vec<<T as PluginCategory>::BoxType>,
}

impl<T: PluginCategory + ?Sized> PluginContainer<T> {
    pub fn plugins(&self) -> &[<T as PluginCategory>::BoxType] {
        &self.plugins
    }
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl<T: PluginCategory + ?Sized> Drop for PluginContainer<T> {
    fn drop(&mut self) {
        self.plugins.clear();
        self.libraries.clear();
    }
}

struct PluginLoader<'a, T: PluginCategory + ?Sized> {
    cfg: &'a Config,
    grammar: &'a Grammar<'a>,
    libraries: Vec<Library>,
    plugins: Vec<<T as PluginCategory>::BoxType>,
}

impl<'a, T: PluginCategory + ?Sized> PluginLoader<'a, T> {
    pub fn new(grammar: &'a Grammar, config: &'a Config) -> PluginLoader<'a, T> {
        PluginLoader {
            cfg: config,
            grammar,
            libraries: Vec::new(),
            plugins: Vec::new(),
        }
    }

    pub fn load(&mut self) -> SudachiResult<()> {
        let configs = <T as PluginCategory>::configurations(self.cfg);
        for cfg in configs {
            let name = extract_plugin_class(cfg)?;
            self.load_plugin(name, cfg)?;
        }
        Ok(())
    }

    pub fn freeze(self) -> PluginContainer<T> {
        return PluginContainer {
            libraries: self.libraries,
            plugins: self.plugins,
        };
    }

    fn load_plugin(&mut self, name: &str, plugin_cfg: &Value) -> SudachiResult<()> {
        let mut plugin = match <T as PluginCategory>::packaged_impl(name) {
            Some(p) => p,
            None => {
                let full_name = self.resolve_plugin_name(name);
                self.load_plugin_from_lib(&full_name)?
            }
        };

        <T as PluginCategory>::do_setup(&mut plugin, plugin_cfg, &self.cfg, &self.grammar)?;
        Ok(())
    }

    fn resolve_plugin_name(&self, name: &str) -> String {
        let resolved = self.cfg.resolve_path(name.to_owned());
        resolved
    }

    fn load_plugin_from_lib(
        &mut self,
        name: &str,
    ) -> SudachiResult<<T as PluginCategory>::BoxType> {
        let lib = unsafe { Library::new(name)? };
        let load_fn: Symbol<fn() -> SudachiResult<<T as PluginCategory>::BoxType>> =
            unsafe { lib.get(b"load_plugin")? };
        let mut plugin = load_fn();

        self.libraries.push(lib);
        plugin
    }
}

fn extract_plugin_class(val: &Value) -> SudachiResult<&str> {
    let obj = match val {
        Value::Object(v) => v,
        _ => {
            return Err(SudachiError::ConfigError(ConfigError::InvalidFormat(
                "plugin config must be an object".to_owned(),
            )));
        }
    };
    match obj.get("class") {
        Some(Value::String(v)) => Ok(v),
        _ => Err(SudachiError::ConfigError(ConfigError::InvalidFormat(
            "plugin config must have 'class' key to indicate plugin SO file".to_owned(),
        ))),
    }
}

pub trait PluginCategory {
    type BoxType;
    type InitFnType;
    fn configurations(cfg: &Config) -> &[Value];
    fn packaged_impl(name: &str) -> Option<Self::BoxType> {
        None
    }
    fn do_setup(
        ptr: &mut Self::BoxType,
        settings: &Value,
        config: &Config,
        grammar: &Grammar,
    ) -> SudachiResult<()>;
}

pub fn load_plugins_of<T: PluginCategory + ?Sized>(
    cfg: &Config,
    grammar: &Grammar,
) -> SudachiResult<PluginContainer<T>> {
    let mut loader: PluginLoader<T> = PluginLoader::new(grammar, cfg);
    loader.load()?;
    Ok(loader.freeze())
}

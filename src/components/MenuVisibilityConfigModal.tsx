import React, { useEffect, useMemo, useState } from 'react';
import { Button, Checkbox, Divider, Modal, Space, Tree, Typography } from 'antd';
import type { TreeProps } from 'antd';
import { useTranslation } from 'react-i18next';
import {
  cloneDefaultMenuVisibility,
  useMenuVisibilityStore,
  type HomeMenuKey,
  type SidebarMenuKey,
} from '../stores/menuVisibilityStore';
import type { MenuVisibilityPreferences } from '../api/tauri';

const { Text } = Typography;

interface MenuVisibilityConfigModalProps {
  open: boolean;
  onClose: () => void;
}

const HOME_MENU_KEYS: HomeMenuKey[] = [
  'connection',
  'dashboard',
  'gh3036',
  'protocol',
  'waveform',
  'system',
];

const SIDEBAR_MENU_KEYS: SidebarMenuKey[] = [
  'home',
  'serial',
  'ble',
  'dashboard',
  'gh3036',
  'protocol',
  'waveform',
  'system',
];

const HOME_TAB_KEYS: Record<HomeMenuKey, string[]> = {
  connection: ['serial', 'ble'],
  dashboard: ['dashboard', 'console', 'settings', 'jsonEditor'],
  gh3036: ['config', 'monitor', 'version', 'factory', 'threshold'],
  protocol: ['editor', 'bind'],
  waveform: ['realtime', 'csvLoader'],
  system: ['info', 'logs', 'settings'],
};

const cloneMenuVisibility = (
  prefs: MenuVisibilityPreferences
): MenuVisibilityPreferences => JSON.parse(JSON.stringify(prefs));

const MenuVisibilityConfigModal: React.FC<MenuVisibilityConfigModalProps> = ({
  open,
  onClose,
}) => {
  const { t } = useTranslation(['home', 'sidebar']);
  const { menuVisibility, saveMenuVisibility, isSaving } = useMenuVisibilityStore();
  const [draft, setDraft] = useState<MenuVisibilityPreferences>(() =>
    cloneMenuVisibility(menuVisibility)
  );

  useEffect(() => {
    if (open) {
      setDraft(cloneMenuVisibility(menuVisibility));
    }
  }, [menuVisibility, open]);

  const homeTreeData: TreeProps['treeData'] = useMemo(
    () =>
      HOME_MENU_KEYS.map((moduleKey) => ({
        key: `home.${moduleKey}`,
        title: t(`home:modules.${moduleKey}.name`),
        children: HOME_TAB_KEYS[moduleKey].map((tabKey) => ({
          key: `home.${moduleKey}.${tabKey}`,
          title: t(`home:modules.${moduleKey}.tabs.${tabKey}`),
        })),
      })),
    [t]
  );

  const checkedHomeKeys = useMemo(() => {
    const keys: string[] = [];
    HOME_MENU_KEYS.forEach((moduleKey) => {
      if (draft.home[moduleKey].visible) {
        keys.push(`home.${moduleKey}`);
      }
      HOME_TAB_KEYS[moduleKey].forEach((tabKey) => {
        if (draft.home[moduleKey].tabs[tabKey]) {
          keys.push(`home.${moduleKey}.${tabKey}`);
        }
      });
    });
    return keys;
  }, [draft]);

  const sidebarOptions = SIDEBAR_MENU_KEYS.map((key) => ({
    label: t(`sidebar:menu.${key}`),
    value: key,
  }));

  const checkedSidebarKeys = SIDEBAR_MENU_KEYS.filter((key) => draft.sidebar[key]);

  const handleHomeCheck: TreeProps['onCheck'] = (checked) => {
    const checkedKeys = new Set(
      Array.isArray(checked) ? checked.map(String) : checked.checked.map(String)
    );
    setDraft((current) => {
      const next = cloneMenuVisibility(current);
      HOME_MENU_KEYS.forEach((moduleKey) => {
        next.home[moduleKey].visible = checkedKeys.has(`home.${moduleKey}`);
        HOME_TAB_KEYS[moduleKey].forEach((tabKey) => {
          next.home[moduleKey].tabs[tabKey] = checkedKeys.has(
            `home.${moduleKey}.${tabKey}`
          );
        });
      });
      return next;
    });
  };

  const handleSidebarChange = (values: Array<string | number | boolean>) => {
    const checkedKeys = new Set(values.map(String));
    setDraft((current) => {
      const next = cloneMenuVisibility(current);
      SIDEBAR_MENU_KEYS.forEach((key) => {
        next.sidebar[key] = checkedKeys.has(key);
      });
      return next;
    });
  };

  const handleSave = async () => {
    await saveMenuVisibility(draft);
    onClose();
  };

  return (
    <Modal
      title="菜单显示配置"
      open={open}
      onCancel={onClose}
      width={640}
      destroyOnClose
      footer={[
        <Button key="reset" onClick={() => setDraft(cloneDefaultMenuVisibility())}>
          恢复默认
        </Button>,
        <Button key="cancel" onClick={onClose}>
          取消
        </Button>,
        <Button key="save" type="primary" loading={isSaving} onClick={handleSave}>
          保存
        </Button>,
      ]}
    >
      <Space direction="vertical" style={{ width: '100%' }} size={12}>
        <div>
          <Text strong>主页面菜单</Text>
          <Tree
            checkable
            checkStrictly
            selectable={false}
            defaultExpandAll
            treeData={homeTreeData}
            checkedKeys={checkedHomeKeys}
            onCheck={handleHomeCheck}
            style={{ marginTop: 8 }}
          />
        </div>
        <Divider style={{ margin: '4px 0' }} />
        <div>
          <Text strong>侧边栏菜单</Text>
          <Checkbox.Group
            options={sidebarOptions}
            value={checkedSidebarKeys}
            onChange={handleSidebarChange}
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
              gap: 8,
              marginTop: 12,
            }}
          />
        </div>
      </Space>
    </Modal>
  );
};

export default MenuVisibilityConfigModal;

import { Layout, Menu, Typography } from 'antd';
import type { MenuProps } from 'antd';
import {
  Navigate,
  Route,
  Routes,
  useLocation,
  useNavigate,
} from 'react-router-dom';
import ExecutionPage from '../features/execution/ExecutionPage';
import ProjectPage from '../features/project/ProjectPage';
import SchemePage from '../features/scheme/SchemePage';
import ProjectSwitcher from '../features/project/ProjectSwitcher';
import TemplatePage from '../features/template/TemplatePage';
import TestItemPage from '../features/testitem/TestItemPage';
import TracePage from '../features/trace/TracePage';

const { Header, Sider, Content } = Layout;

const menuItems: MenuProps['items'] = [
  { key: '/', label: '需求追踪' },
  { key: '/templates', label: '文档模板' },
  { key: '/executions', label: '执行清单' },
  { key: '/test-items', label: '测试项编写' },
  { key: '/schemes', label: '台账编排' },
  { key: '/projects', label: '项目管理' },
];

export default function App() {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  return (
    <Layout className="h-screen! overflow-hidden!">
      <Header className="flex items-center justify-between">
        <Typography.Title level={4} className="m-0! text-white!">
          MBSE 测试子系统
        </Typography.Title>
        <ProjectSwitcher />
      </Header>
      <Layout className="min-h-0!">
        <Sider width={160} theme="light">
          <Menu
            mode="inline"
            items={menuItems}
            selectedKeys={[pathname]}
            onClick={({ key }) => navigate(key)}
            className="h-full"
          />
        </Sider>
        <Content className="overflow-auto p-4">
          <Routes>
            <Route path="/" element={<TracePage />} />
            <Route path="/templates" element={<TemplatePage />} />
            <Route path="/executions" element={<ExecutionPage />} />
            <Route path="/test-items" element={<TestItemPage />} />
            <Route path="/schemes" element={<SchemePage />} />
            <Route path="/projects" element={<ProjectPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </Content>
      </Layout>
    </Layout>
  );
}

import { BrowserRouter, Navigate, Route, Routes } from 'react-router'

import { BranchConfigProvider } from './branch-config/BranchConfigProvider.js'
import { CustomerEntryScreen } from './customer-flow/screens/CustomerEntryScreen.js'

function App() {
  return (
    <BranchConfigProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Navigate replace to="/customer" />} />
          <Route path="/customer" element={<CustomerEntryScreen />} />
        </Routes>
      </BrowserRouter>
    </BranchConfigProvider>
  )
}

export default App

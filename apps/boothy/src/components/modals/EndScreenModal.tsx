import { motion, AnimatePresence } from 'framer-motion';
import { Heart } from 'lucide-react';
import Button from '../ui/Button';

interface EndScreenModalProps {
  isAdmin: boolean;
  isOpen: boolean;
  message: string;
  onExit(): void;
}

export default function EndScreenModal({ isAdmin, isOpen, message, onExit }: EndScreenModalProps) {
  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.4 }}
          aria-modal="true"
          className="fixed inset-0 z-[60] flex items-center justify-center bg-black/80 backdrop-blur-md"
          role="dialog"
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.98 }}
            transition={{ duration: 0.3, ease: 'easeOut' }}
            className="max-w-xl px-12 py-10 bg-surface/90 backdrop-blur-xl rounded-2xl shadow-2xl text-center border border-accent/10"
          >
            {/* Simple accent heart icon */}
            <div className="mb-6 flex justify-center">
              <Heart size={48} className="text-accent" strokeWidth={1.5} />
            </div>

            {/* Main message */}
            <p className="text-2xl font-semibold text-text-primary whitespace-pre-wrap leading-relaxed">{message}</p>

            {/* Admin exit button */}
            {isAdmin && (
              <div className="mt-8 flex justify-center">
                <Button onClick={onExit}>Exit End Screen</Button>
              </div>
            )}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

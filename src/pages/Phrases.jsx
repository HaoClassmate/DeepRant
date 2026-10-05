import React from 'react';
import { motion } from 'framer-motion';
import { useStore } from '../components/StoreProvider';

export default function Phrases() {
    const { settings } = useStore();
    const phrases = settings?.phrases || [];

    return (
        <div className="h-full flex flex-col gap-6 p-6 ">
            <motion.div
                className="w-full bg-white dark:bg-zinc-900 rounded-2xl p-6 border border-zinc-200 dark:border-zinc-800 shadow-[0_8px_30px_rgb(0,0,0,0.04)] dark:shadow-[0_8px_30px_rgb(0,0,0,0.1)] backdrop-blur-sm"
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
            >
                <h1 className="text-2xl font-bold text-zinc-900 dark:text-white mb-6">常用语</h1>
                <div className="overflow-auto">
                    <table className="min-w-full">
                        <thead>
                            <tr className="border-b border-zinc-200 dark:border-zinc-800">
                                <th className="py-3 text-left text-sm font-medium text-zinc-500 dark:text-zinc-400">
                                    文字
                                </th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-zinc-200 dark:divide-zinc-800">
                            {phrases.map((item) => (
                                <tr key={item.id} className="hover:bg-zinc-50 dark:hover:bg-zinc-800/50 transition-colors">
                                    <td className="py-4 text-zinc-900 dark:text-white">
                                        {item.phrase}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            </motion.div>
            <div className="h-12 flex flex-col gap-6 p-6 ">

            </div>
        </div>
    );
} 